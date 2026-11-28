"""RunPod `TRL` SFT + Verifiable-Reward GRPO Trainer (`train_grpo`).

Trains an open-weight Apache-2.0 Digimon Tactician (`Qwen/Qwen3-8B` default;
`Qwen/Qwen3-14B` supported) in two sequential stages on a single RunPod GPU:

1. **Stage A (`SFTTrainer`)**: Supervised warm-start on `<think>...</think>` +
   JSON completions from `sft_train.jsonl` using 4-bit NF4 QLoRA
   (`lora_r=32`, `lora_alpha=64`).
2. **Stage B (`GRPOTrainer`)**: Group Relative Policy Optimization over
   `grpo_prompts.jsonl` wired directly to in-process `DigimonVerifiableReward`
   (`RustDebugGame` + `RustHeadlessGame` + `CardAdmissibility`).

Zero Heavy Import Guarantee:
`torch`, `transformers`, `peft`, `trl`, `datasets`, and `bitsandbytes` are
imported strictly lazily inside `_load_training_libraries()` only when
`dry_run=False`, so importing `tools.tactician_grpo` and running `--dry-run` in
standard CI requires zero GPU / LLM training packages.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass
import inspect
import json
from pathlib import Path
from typing import Any, Mapping, Optional

from .verifiable_reward import (
    TASK_COMPOSE_COUNTER_PUZZLE,
    TASK_SOLVE_POSITION,
    DigimonVerifiableReward,
)

DEFAULT_BASE_MODEL = "Qwen/Qwen3-8B"

MODEL_PRESETS: dict[str, dict[str, Any]] = {
    "Qwen/Qwen3-8B": {
        "license": "Apache-2.0",
        "params_b": 8.0,
        "vram_q4_k_m_gb": 5.2,
        "co_resides_with_ppo_on_12gb_rtx4070": True,
        "recommended_runpod_gpu": "1x RTX 4090 (24 GB) or A100 (40 GB)",
        "recommended_ollama_tag": "digimon-tactician:8b",
        "default_gguf_filename": "digimon-tactician-qwen3-8b-q4_k_m.gguf",
    },
    "Qwen/Qwen3-14B": {
        "license": "Apache-2.0",
        "params_b": 14.8,
        "vram_q4_k_m_gb": 9.0,
        "co_resides_with_ppo_on_12gb_rtx4070": False,
        "recommended_runpod_gpu": "1x A100 (40/80 GB) or RTX A6000 (48 GB)",
        "recommended_ollama_tag": "digimon-tactician:14b",
        "default_gguf_filename": "digimon-tactician-qwen3-14b-q4_k_m.gguf",
    },
}

SUPPORTED_BASE_MODELS = tuple(MODEL_PRESETS.keys())

_BLOCKED_CLOSED_PREFIXES = ("gpt-", "o1-", "o3-", "claude-", "gemini-")


@dataclass
class TacticianTrainConfig:
    """Configuration for 2-stage QLoRA `SFTTrainer` -> `GRPOTrainer` training."""

    base_model: str = DEFAULT_BASE_MODEL
    sft_dataset_path: Optional[Path] = None
    grpo_dataset_path: Optional[Path] = None
    output_dir: Path = Path("runs/tactician_grpo")

    # 4-bit NF4 QLoRA hyperparameters
    load_in_4bit: bool = True
    lora_r: int = 32
    lora_alpha: int = 64
    lora_dropout: float = 0.05
    target_modules: tuple[str, ...] = (
        "q_proj",
        "k_proj",
        "v_proj",
        "o_proj",
        "gate_proj",
        "up_proj",
        "down_proj",
    )

    # Stage A: SFT settings
    skip_sft: bool = False
    sft_epochs: int = 2
    sft_learning_rate: float = 2e-4
    sft_batch_size: int = 1
    sft_grad_accum: int = 8
    sft_max_seq_length: int = 8192

    # Stage B: GRPO settings
    skip_grpo: bool = False
    grpo_max_steps: int = 200
    grpo_learning_rate: float = 5e-6
    grpo_beta: float = 0.04
    grpo_num_generations: int = 8
    grpo_batch_size: int = 1
    grpo_grad_accum: int = 8
    grpo_max_prompt_length: int = 6144
    grpo_max_completion_length: int = 2048

    # Verifiable reward engine settings
    adversary_seeds: int = 2
    admissibility_path: Optional[Path] = None
    seed: int = 42


def resolve_model_preset(base_model: str) -> dict[str, Any]:
    """Return VRAM/Ollama metadata for `Qwen/Qwen3-8B`, `Qwen/Qwen3-14B`, or custom open checkpoints."""
    cleaned = str(base_model or "").strip()
    if cleaned in MODEL_PRESETS:
        return dict(MODEL_PRESETS[cleaned])
    lower = cleaned.lower()
    if "14b" in lower:
        preset = dict(MODEL_PRESETS["Qwen/Qwen3-14B"])
        preset["custom_model"] = True
        return preset
    preset = dict(MODEL_PRESETS["Qwen/Qwen3-8B"])
    preset["custom_model"] = cleaned != DEFAULT_BASE_MODEL
    return preset


def _validate_jsonl_datasets(
    sft_path: Optional[Path],
    grpo_path: Optional[Path],
    *,
    admissibility_path: Optional[Path] = None,
) -> dict[str, Any]:
    """Validate optional SFT & GRPO JSONL files and run a 1-row reward smoke check."""
    stats: dict[str, Any] = {
        "sft_validated": False,
        "sft_rows": 0,
        "grpo_validated": False,
        "grpo_rows": 0,
        "task_counts": {TASK_SOLVE_POSITION: 0, TASK_COMPOSE_COUNTER_PUZZLE: 0},
        "smoke_reward_score": None,
    }
    sft_by_id: dict[str, dict[str, Any]] = {}

    if sft_path is not None:
        p = Path(sft_path)
        if not p.exists():
            raise FileNotFoundError(f"SFT dataset file does not exist: {p}")
        lines = [ln for ln in p.read_text(encoding="utf-8").splitlines() if ln.strip()]
        if not lines:
            raise ValueError(f"SFT dataset file is empty: {p}")
        for idx, raw_ln in enumerate(lines):
            row = json.loads(raw_ln)
            msgs = row.get("messages")
            if not isinstance(msgs, list) or len(msgs) < 2:
                raise ValueError(f"sft_train.jsonl row {idx} missing valid 'messages' list")
            roles = [m.get("role") for m in msgs if isinstance(m, Mapping)]
            if "user" not in roles or "assistant" not in roles:
                raise ValueError(f"sft_train.jsonl row {idx} must include 'user' and 'assistant' messages")
            if row.get("id"):
                sft_by_id[str(row["id"])] = row
        stats["sft_validated"] = True
        stats["sft_rows"] = len(lines)

    if grpo_path is not None:
        p = Path(grpo_path)
        if not p.exists():
            raise FileNotFoundError(f"GRPO prompts file does not exist: {p}")
        lines = [ln for ln in p.read_text(encoding="utf-8").splitlines() if ln.strip()]
        if not lines:
            raise ValueError(f"GRPO prompts file is empty: {p}")
        first_matched_pair: Optional[tuple[dict[str, Any], dict[str, Any]]] = None
        for idx, raw_ln in enumerate(lines):
            row = json.loads(raw_ln)
            prompt = row.get("prompt")
            if not isinstance(prompt, list) or not prompt:
                raise ValueError(f"grpo_prompts.jsonl row {idx} missing conversational 'prompt' list")
            tk = str(row.get("task_kind") or "")
            if tk in stats["task_counts"]:
                stats["task_counts"][tk] += 1
            if "ground_truth_meta" not in row:
                raise ValueError(f"grpo_prompts.jsonl row {idx} missing 'ground_truth_meta'")
            rid = str(row.get("id") or "")
            if first_matched_pair is None and rid in sft_by_id:
                first_matched_pair = (sft_by_id[rid], row)
        stats["grpo_validated"] = True
        stats["grpo_rows"] = len(lines)

        if first_matched_pair is not None:
            sft_sample, grpo_sample = first_matched_pair
            assistant_msg = next(
                m["content"] for m in sft_sample["messages"] if m.get("role") == "assistant"
            )
            reward_fn = DigimonVerifiableReward(
                admissibility_path=admissibility_path,
                adversary_seeds=1,
            )
            scores = reward_fn(
                prompts=[grpo_sample["prompt"]],
                completions=[[{"role": "assistant", "content": assistant_msg}]],
                ground_truth_meta=[grpo_sample["ground_truth_meta"]],
                task_kind=[grpo_sample["task_kind"]],
            )
            stats["smoke_reward_score"] = scores[0]

    return stats


def validate_train_config(config: TacticianTrainConfig) -> dict[str, Any]:
    """Validate `TacticianTrainConfig` without importing GPU / LLM training packages."""
    model_id = str(config.base_model or "").strip()
    if not model_id:
        raise ValueError("base_model must be non-empty")
    lower = model_id.lower()
    if any(lower.startswith(pref) for pref in _BLOCKED_CLOSED_PREFIXES):
        raise PermissionError(
            f"Closed-weight model {model_id!r} cannot be used in tactician_grpo; "
            f"use Apache-2.0 open-weight models such as {SUPPORTED_BASE_MODELS}."
        )

    if int(config.lora_r) <= 0:
        raise ValueError(f"lora_r must be > 0, got {config.lora_r}")
    if int(config.lora_alpha) <= 0:
        raise ValueError(f"lora_alpha must be > 0, got {config.lora_alpha}")
    if not (0.0 <= float(config.lora_dropout) < 1.0):
        raise ValueError(f"lora_dropout must lie in [0.0, 1.0), got {config.lora_dropout}")
    if not config.target_modules:
        raise ValueError("target_modules must be non-empty")

    if config.skip_sft and config.skip_grpo:
        raise ValueError("Cannot set both skip_sft=True and skip_grpo=True")
    if int(config.grpo_num_generations) < 2:
        raise ValueError(f"grpo_num_generations must be >= 2 for group-relative advantage, got {config.grpo_num_generations}")
    if int(config.sft_max_seq_length) < 512:
        raise ValueError(f"sft_max_seq_length must be >= 512, got {config.sft_max_seq_length}")
    if int(config.grpo_max_prompt_length) < 256 or int(config.grpo_max_completion_length) < 128:
        raise ValueError("grpo_max_prompt_length >= 256 and grpo_max_completion_length >= 128 required")

    preset = resolve_model_preset(model_id)
    ds_stats = _validate_jsonl_datasets(
        config.sft_dataset_path,
        config.grpo_dataset_path,
        admissibility_path=config.admissibility_path,
    )

    cfg_dict = asdict(config)
    for path_key in ("sft_dataset_path", "grpo_dataset_path", "output_dir", "admissibility_path"):
        if cfg_dict.get(path_key) is not None:
            cfg_dict[path_key] = str(cfg_dict[path_key])
    cfg_dict["target_modules"] = list(config.target_modules)

    return {
        "schema": "tactician-grpo-plan-v1",
        "ok": True,
        "base_model": model_id,
        "model_preset": preset,
        "qlora": {
            "load_in_4bit": bool(config.load_in_4bit),
            "quant_type": "nf4" if config.load_in_4bit else "none",
            "compute_dtype": "bfloat16",
            "lora_r": int(config.lora_r),
            "lora_alpha": int(config.lora_alpha),
            "lora_dropout": float(config.lora_dropout),
            "target_modules": list(config.target_modules),
        },
        "stages": {
            "sft": {
                "enabled": not config.skip_sft,
                "trainer": "trl.SFTTrainer",
                "epochs": int(config.sft_epochs),
                "learning_rate": float(config.sft_learning_rate),
                "per_device_train_batch_size": int(config.sft_batch_size),
                "gradient_accumulation_steps": int(config.sft_grad_accum),
                "max_seq_length": int(config.sft_max_seq_length),
                "adapter_dir": str(Path(config.output_dir) / "sft_adapter"),
            },
            "grpo": {
                "enabled": not config.skip_grpo,
                "trainer": "trl.GRPOTrainer",
                "reward_callable": "DigimonVerifiableReward",
                "max_steps": int(config.grpo_max_steps),
                "learning_rate": float(config.grpo_learning_rate),
                "beta": float(config.grpo_beta),
                "num_generations": int(config.grpo_num_generations),
                "per_device_train_batch_size": int(config.grpo_batch_size),
                "gradient_accumulation_steps": int(config.grpo_grad_accum),
                "max_prompt_length": int(config.grpo_max_prompt_length),
                "max_completion_length": int(config.grpo_max_completion_length),
                "adapter_dir": str(Path(config.output_dir) / "grpo_adapter"),
            },
        },
        "dataset_validation": ds_stats,
        "config": cfg_dict,
    }


def _load_training_libraries() -> dict[str, Any]:
    """Lazily import heavy GPU training packages (`torch`, `transformers`, `peft`, `trl`, `datasets`)."""
    try:
        import datasets
        import peft
        import torch
        import transformers
        import trl
    except ImportError as exc:
        raise RuntimeError(
            "RunPod SFT + GRPO execution requires GPU training dependencies "
            "(`pip install torch transformers peft trl datasets bitsandbytes accelerate`). "
            "Use `--dry-run` to validate configs and datasets without GPU packages."
        ) from exc

    bnb_cls = getattr(transformers, "BitsAndBytesConfig", None)
    return {
        "torch": torch,
        "transformers": transformers,
        "peft": peft,
        "trl": trl,
        "datasets": datasets,
        "BitsAndBytesConfig": bnb_cls,
    }


def _load_jsonl_as_hf_dataset(datasets_mod: Any, path: Path, *, stringify_meta: bool = False) -> Any:
    rows: list[dict[str, Any]] = []
    for ln in Path(path).read_text(encoding="utf-8").splitlines():
        if not ln.strip():
            continue
        row = json.loads(ln)
        if stringify_meta and isinstance(row.get("ground_truth_meta"), Mapping):
            row["ground_truth_meta"] = json.dumps(row["ground_truth_meta"], ensure_ascii=False)
        rows.append(row)
    return datasets_mod.Dataset.from_list(rows)


def _filter_supported_kwargs(callable_obj: Any, kwargs: Mapping[str, Any]) -> dict[str, Any]:
    sig = inspect.signature(callable_obj)
    if any(p.kind == inspect.Parameter.VAR_KEYWORD for p in sig.parameters.values()):
        return dict(kwargs)
    return {k: v for k, v in kwargs.items() if k in sig.parameters}


def run_sft_and_grpo(
    config: TacticianTrainConfig,
    *,
    dry_run: bool = False,
    write_plan: bool = True,
) -> dict[str, Any]:
    """Validate configuration and execute `--dry-run` or full RunPod SFT -> GRPO training."""
    plan = validate_train_config(config)
    out_dir = Path(config.output_dir)

    if dry_run:
        if write_plan:
            out_dir.mkdir(parents=True, exist_ok=True)
            plan_path = out_dir / "dry_run_plan.json"
            plan_path.write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
            plan["plan_path"] = str(plan_path)
        return plan

    if not config.skip_sft and config.sft_dataset_path is None:
        raise ValueError("sft_dataset_path is required when skip_sft=False and dry_run=False")
    if not config.skip_grpo and config.grpo_dataset_path is None:
        raise ValueError("grpo_dataset_path is required when skip_grpo=False and dry_run=False")

    libs = _load_training_libraries()
    torch = libs["torch"]
    transformers = libs["transformers"]
    peft = libs["peft"]
    trl = libs["trl"]
    datasets_mod = libs["datasets"]
    bnb_cls = libs["BitsAndBytesConfig"]

    out_dir.mkdir(parents=True, exist_ok=True)
    quant_config = None
    if config.load_in_4bit and bnb_cls is not None:
        quant_config = bnb_cls(
            load_in_4bit=True,
            bnb_4bit_quant_type="nf4",
            bnb_4bit_compute_dtype=torch.bfloat16,
            bnb_4bit_use_double_quant=True,
        )

    tokenizer = transformers.AutoTokenizer.from_pretrained(config.base_model, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    lora_config = peft.LoraConfig(
        r=int(config.lora_r),
        lora_alpha=int(config.lora_alpha),
        lora_dropout=float(config.lora_dropout),
        bias="none",
        task_type="CAUSAL_LM",
        target_modules=list(config.target_modules),
    )

    sft_adapter_dir = out_dir / "sft_adapter"
    grpo_adapter_dir = out_dir / "grpo_adapter"

    # Stage A: SFT Warm-Start
    if not config.skip_sft:
        assert config.sft_dataset_path is not None
        sft_ds = _load_jsonl_as_hf_dataset(datasets_mod, Path(config.sft_dataset_path))
        sft_model = transformers.AutoModelForCausalLM.from_pretrained(
            config.base_model,
            quantization_config=quant_config,
            torch_dtype=torch.bfloat16,
            device_map="auto",
            trust_remote_code=True,
        )

        sft_cfg_cls = trl.SFTConfig
        raw_sft_kwargs: dict[str, Any] = {
            "output_dir": str(sft_adapter_dir),
            "num_train_epochs": int(config.sft_epochs),
            "learning_rate": float(config.sft_learning_rate),
            "per_device_train_batch_size": int(config.sft_batch_size),
            "gradient_accumulation_steps": int(config.sft_grad_accum),
            "max_length": int(config.sft_max_seq_length),
            "max_seq_length": int(config.sft_max_seq_length),
            "bf16": torch.cuda.is_available(),
            "logging_steps": 5,
            "save_strategy": "epoch",
            "seed": int(config.seed),
            "report_to": "none",
        }
        sft_args = sft_cfg_cls(**_filter_supported_kwargs(sft_cfg_cls.__init__, raw_sft_kwargs))
        trainer_kwargs = _filter_supported_kwargs(
            trl.SFTTrainer.__init__,
            {
                "model": sft_model,
                "args": sft_args,
                "train_dataset": sft_ds,
                "peft_config": lora_config,
                "processing_class": tokenizer,
                "tokenizer": tokenizer,
            },
        )
        sft_trainer = trl.SFTTrainer(**trainer_kwargs)
        sft_trainer.train()
        sft_trainer.save_model(str(sft_adapter_dir))
        tokenizer.save_pretrained(str(sft_adapter_dir))
        del sft_trainer, sft_model
        if torch.cuda.is_available():
            torch.cuda.empty_cache()

    # Stage B: Verifiable-Reward GRPO
    if not config.skip_grpo:
        assert config.grpo_dataset_path is not None
        # Serialize `ground_truth_meta` as JSON strings so HF Arrow tables never drop nested keys;
        # `DigimonVerifiableReward` transparently parses both JSON strings and dicts!
        grpo_ds = _load_jsonl_as_hf_dataset(
            datasets_mod,
            Path(config.grpo_dataset_path),
            stringify_meta=True,
        )
        base_for_grpo = transformers.AutoModelForCausalLM.from_pretrained(
            config.base_model,
            quantization_config=quant_config,
            torch_dtype=torch.bfloat16,
            device_map="auto",
            trust_remote_code=True,
        )
        if not config.skip_sft and sft_adapter_dir.exists():
            grpo_model = peft.PeftModel.from_pretrained(
                base_for_grpo,
                str(sft_adapter_dir),
                is_trainable=True,
            )
            grpo_peft_cfg = None
        else:
            grpo_model = base_for_grpo
            grpo_peft_cfg = lora_config

        reward_fn = DigimonVerifiableReward(
            admissibility_path=config.admissibility_path,
            adversary_seeds=int(config.adversary_seeds),
        )
        grpo_cfg_cls = trl.GRPOConfig
        raw_grpo_kwargs: dict[str, Any] = {
            "output_dir": str(grpo_adapter_dir),
            "max_steps": int(config.grpo_max_steps),
            "learning_rate": float(config.grpo_learning_rate),
            "beta": float(config.grpo_beta),
            "num_generations": int(config.grpo_num_generations),
            "per_device_train_batch_size": int(config.grpo_batch_size),
            "gradient_accumulation_steps": int(config.grpo_grad_accum),
            "max_prompt_length": int(config.grpo_max_prompt_length),
            "max_completion_length": int(config.grpo_max_completion_length),
            "bf16": torch.cuda.is_available(),
            "logging_steps": 1,
            "save_steps": 50,
            "seed": int(config.seed),
            "report_to": "none",
        }
        grpo_args = grpo_cfg_cls(**_filter_supported_kwargs(grpo_cfg_cls.__init__, raw_grpo_kwargs))
        grpo_trainer_kwargs: dict[str, Any] = {
            "model": grpo_model,
            "reward_funcs": [reward_fn],
            "args": grpo_args,
            "train_dataset": grpo_ds,
            "processing_class": tokenizer,
            "tokenizer": tokenizer,
        }
        if grpo_peft_cfg is not None:
            grpo_trainer_kwargs["peft_config"] = grpo_peft_cfg

        grpo_trainer = trl.GRPOTrainer(
            **_filter_supported_kwargs(trl.GRPOTrainer.__init__, grpo_trainer_kwargs)
        )
        grpo_trainer.train()
        grpo_trainer.save_model(str(grpo_adapter_dir))
        tokenizer.save_pretrained(str(grpo_adapter_dir))

    manifest = {
        **plan,
        "executed": True,
        "sft_adapter_dir": str(sft_adapter_dir) if not config.skip_sft else None,
        "grpo_adapter_dir": str(grpo_adapter_dir) if not config.skip_grpo else None,
    }
    manifest_path = out_dir / "train_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    manifest["manifest_path"] = str(manifest_path)
    return manifest
