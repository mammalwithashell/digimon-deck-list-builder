"""CLI Entrypoint for `tools.tactician_grpo`.

Usage:
  PYTHONPATH=code python -m tools.tactician_grpo {export-dataset,score-completion,train-dry-run,train,export-ollama}
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Optional, Sequence

from .dataset_exporter import export_tactician_datasets
from .export_ollama import OllamaExportConfig, export_ollama_bundle
from .train_grpo import DEFAULT_BASE_MODEL, TacticianTrainConfig, run_sft_and_grpo
from .verifiable_reward import score_single_completion


def _add_train_args(parser: argparse.ArgumentParser, *, force_dry_run: bool = False) -> None:
    parser.add_argument(
        "--base-model",
        default=DEFAULT_BASE_MODEL,
        help=f"Open-weight Apache-2.0 HF model ID or local path (default: {DEFAULT_BASE_MODEL}).",
    )
    parser.add_argument("--sft-dataset", type=Path, default=None, help="Path to sft_train.jsonl.")
    parser.add_argument("--grpo-dataset", type=Path, default=None, help="Path to grpo_prompts.jsonl.")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("runs/tactician_grpo"),
        help="Output directory for LoRA adapters and training manifests.",
    )
    parser.add_argument("--lora-r", type=int, default=32)
    parser.add_argument("--lora-alpha", type=int, default=64)
    parser.add_argument("--lora-dropout", type=float, default=0.05)
    parser.add_argument("--no-4bit", action="store_true", help="Disable 4-bit NF4 QLoRA quantization.")
    parser.add_argument("--skip-sft", action="store_true", help="Skip Stage A SFT warm-start.")
    parser.add_argument("--skip-grpo", action="store_true", help="Skip Stage B verifiable-reward GRPO.")
    parser.add_argument("--sft-epochs", type=int, default=2)
    parser.add_argument("--sft-lr", type=float, default=2e-4)
    parser.add_argument("--sft-batch-size", type=int, default=1)
    parser.add_argument("--sft-grad-accum", type=int, default=8)
    parser.add_argument("--sft-max-seq-length", type=int, default=8192)
    parser.add_argument("--grpo-max-steps", type=int, default=200)
    parser.add_argument("--grpo-lr", type=float, default=5e-6)
    parser.add_argument("--grpo-beta", type=float, default=0.04)
    parser.add_argument("--grpo-num-generations", type=int, default=8)
    parser.add_argument("--grpo-batch-size", type=int, default=1)
    parser.add_argument("--grpo-grad-accum", type=int, default=8)
    parser.add_argument("--grpo-max-prompt-length", type=int, default=6144)
    parser.add_argument("--grpo-max-completion-length", type=int, default=2048)
    parser.add_argument("--adversary-seeds", type=int, default=2)
    parser.add_argument("--admissibility", type=Path, default=None)
    parser.add_argument("--seed", type=int, default=42)
    if not force_dry_run:
        parser.add_argument(
            "--dry-run",
            action="store_true",
            help="Validate training config & JSONL datasets without requiring GPUs or TRL/PEFT.",
        )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python -m tools.tactician_grpo",
        description="RunPod SFT + Verifiable-Reward GRPO -> Local RTX 4070 Ollama Tactician Pipeline.",
    )
    sub = parser.add_subparsers(dest="subcommand", required=True)

    # 1. export-dataset
    p_export = sub.add_parser(
        "export-dataset",
        help="Export sft_train.jsonl and grpo_prompts.jsonl from qa/puzzles/ and qa/search-exams/.",
    )
    p_export.add_argument(
        "--out-dir",
        type=Path,
        default=Path("runs/tactician_grpo/datasets"),
        help="Destination directory for sft_train.jsonl and grpo_prompts.jsonl.",
    )
    p_export.add_argument("--puzzles-dir", type=Path, default=None)
    p_export.add_argument("--search-exams-dir", type=Path, default=None)
    p_export.add_argument("--cards-json", type=Path, default=None)
    p_export.add_argument("--rules-context", type=Path, default=None)
    p_export.add_argument("--no-deduplicate", action="store_true")

    # 2. score-completion
    p_score = sub.add_parser(
        "score-completion",
        help="Evaluate a single puzzle/solve completion across all 5 verifiable engine reward stages.",
    )
    p_score.add_argument("--completion-file", type=Path, default=None, help="Path to completion text/JSON file.")
    p_score.add_argument("--completion-text", default=None, help="Inline completion text/JSON.")
    p_score.add_argument("--meta-file", type=Path, default=None, help="Optional ground_truth_meta JSON file.")
    p_score.add_argument("--meta-json", default=None, help="Optional inline ground_truth_meta JSON string.")
    p_score.add_argument("--task-kind", default=None, choices=["solve_position", "compose_counter_puzzle"])
    p_score.add_argument("--seeds", type=int, default=2, help="Number of adversary seeds for Stage 4/5.")
    p_score.add_argument("--admissibility", type=Path, default=None)
    p_score.add_argument("--use-mcp", action="store_true", help="Verify Stages 4/5 through digimon-engine-mcp.")

    # 3. train-dry-run
    p_dry = sub.add_parser(
        "train-dry-run",
        help="Validate QLoRA SFT + GRPO config and optional JSONL datasets without GPUs.",
    )
    _add_train_args(p_dry, force_dry_run=True)

    # 4. train
    p_train = sub.add_parser(
        "train",
        help="Run 2-stage QLoRA SFTTrainer -> GRPOTrainer (or --dry-run config validation).",
    )
    _add_train_args(p_train, force_dry_run=False)

    # 5. export-ollama
    p_ollama = sub.add_parser(
        "export-ollama",
        help="Generate Ollama Modelfile (Q4_K_M, num_ctx 16384) and local RTX 4070 helper commands.",
    )
    p_ollama.add_argument(
        "--from-source",
        default="./digimon-tactician-qwen3-8b-q4_k_m.gguf",
        help="Quantized GGUF file path or HF checkpoint directory for Modelfile FROM.",
    )
    p_ollama.add_argument("--base-model", default=DEFAULT_BASE_MODEL)
    p_ollama.add_argument("--model-tag", default=None, help="Ollama tag (default: digimon-tactician:8b or :14b).")
    p_ollama.add_argument("--adapter-dir", type=Path, default=None, help="Optional unmerged LoRA adapter directory.")
    p_ollama.add_argument(
        "--out-dir",
        type=Path,
        default=Path("runs/tactician_grpo/ollama"),
        help="Destination directory for Modelfile, deploy_ollama.sh, and export_manifest.json.",
    )
    p_ollama.add_argument("--quantization", default="Q4_K_M")
    p_ollama.add_argument("--temperature", type=float, default=0.2)
    p_ollama.add_argument("--top-p", type=float, default=0.9)
    p_ollama.add_argument("--repeat-penalty", type=float, default=1.05)
    p_ollama.add_argument("--num-ctx", type=int, default=16384)
    p_ollama.add_argument("--num-predict", type=int, default=4096)

    return parser


def _build_train_config(args: argparse.Namespace) -> TacticianTrainConfig:
    return TacticianTrainConfig(
        base_model=args.base_model,
        sft_dataset_path=args.sft_dataset,
        grpo_dataset_path=args.grpo_dataset,
        output_dir=args.output_dir,
        load_in_4bit=not bool(args.no_4bit),
        lora_r=int(args.lora_r),
        lora_alpha=int(args.lora_alpha),
        lora_dropout=float(args.lora_dropout),
        skip_sft=bool(args.skip_sft),
        skip_grpo=bool(args.skip_grpo),
        sft_epochs=int(args.sft_epochs),
        sft_learning_rate=float(args.sft_lr),
        sft_batch_size=int(args.sft_batch_size),
        sft_grad_accum=int(args.sft_grad_accum),
        sft_max_seq_length=int(args.sft_max_seq_length),
        grpo_max_steps=int(args.grpo_max_steps),
        grpo_learning_rate=float(args.grpo_lr),
        grpo_beta=float(args.grpo_beta),
        grpo_num_generations=int(args.grpo_num_generations),
        grpo_batch_size=int(args.grpo_batch_size),
        grpo_grad_accum=int(args.grpo_grad_accum),
        grpo_max_prompt_length=int(args.grpo_max_prompt_length),
        grpo_max_completion_length=int(args.grpo_max_completion_length),
        adversary_seeds=int(args.adversary_seeds),
        admissibility_path=args.admissibility,
        seed=int(args.seed),
    )


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.subcommand == "export-dataset":
        summary = export_tactician_datasets(
            out_dir=args.out_dir,
            puzzles_dir=args.puzzles_dir,
            search_exams_dir=args.search_exams_dir,
            cards_json_path=args.cards_json,
            rules_context_path=args.rules_context,
            deduplicate_ids=not bool(args.no_deduplicate),
        )
        print(json.dumps(summary.to_dict(), indent=2))
        return 0

    if args.subcommand == "score-completion":
        if args.completion_file is not None:
            comp_text = Path(args.completion_file).read_text(encoding="utf-8")
        elif args.completion_text is not None:
            comp_text = str(args.completion_text)
        else:
            comp_text = sys.stdin.read()

        meta: Optional[str] = None
        if args.meta_file is not None:
            meta = Path(args.meta_file).read_text(encoding="utf-8")
        elif args.meta_json is not None:
            meta = str(args.meta_json)

        if args.use_mcp:
            from tools.puzzle_lab.mcp_client import StdioMcpClient

            with StdioMcpClient.for_engine_mcp() as mcp:
                bd = score_single_completion(
                    comp_text,
                    ground_truth_meta=meta,
                    task_kind=args.task_kind,
                    admissibility_path=args.admissibility,
                    mcp_client=mcp,
                    adversary_seeds=int(args.seeds),
                )
        else:
            bd = score_single_completion(
                comp_text,
                ground_truth_meta=meta,
                task_kind=args.task_kind,
                admissibility_path=args.admissibility,
                adversary_seeds=int(args.seeds),
            )
        print(json.dumps(bd.to_dict(), indent=2))
        return 0

    if args.subcommand in ("train-dry-run", "train"):
        cfg = _build_train_config(args)
        is_dry = args.subcommand == "train-dry-run" or bool(getattr(args, "dry_run", False))
        result = run_sft_and_grpo(cfg, dry_run=is_dry)
        print(json.dumps(result, indent=2))
        return 0

    if args.subcommand == "export-ollama":
        ollama_cfg = OllamaExportConfig(
            from_source=args.from_source,
            base_model=args.base_model,
            model_tag=args.model_tag,
            adapter_dir=args.adapter_dir,
            quantization=args.quantization,
            temperature=float(args.temperature),
            top_p=float(args.top_p),
            repeat_penalty=float(args.repeat_penalty),
            num_ctx=int(args.num_ctx),
            num_predict=int(args.num_predict),
        )
        res = export_ollama_bundle(ollama_cfg, out_dir=args.out_dir)
        print(json.dumps(res.to_dict(), indent=2))
        return 0

    parser.error(f"Unknown subcommand: {args.subcommand}")
    return 2
