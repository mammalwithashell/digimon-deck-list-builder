"""RTX 4070 `Ollama` Exporter (`export_ollama`).

Generates a ready-to-run Ollama `Modelfile` (`Q4_K_M`, `num_ctx 16384`,
temperature/sampling parameters, and Digimon Tactician `SYSTEM` prompt) plus
quantization & deployment helper commands (`llama-quantize ... Q4_K_M`,
`ollama create digimon-tactician:8b -f Modelfile`) ready to plug directly into
`python -m tools.puzzle_lab --provider ollama --author-model digimon-tactician:8b`.
"""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any, Optional

from .dataset_exporter import TACTICIAN_SYSTEM_PROMPT
from .train_grpo import DEFAULT_BASE_MODEL, resolve_model_preset

DEFAULT_OLLAMA_MODEL_TAG = "digimon-tactician:8b"


@dataclass
class OllamaExportConfig:
    """Configuration for generating an Ollama `Modelfile` and quantization bundle."""

    from_source: str | Path = "./digimon-tactician-qwen3-8b-q4_k_m.gguf"
    base_model: str = DEFAULT_BASE_MODEL
    model_tag: Optional[str] = None
    adapter_dir: Optional[Path] = None
    quantization: str = "Q4_K_M"
    temperature: float = 0.2
    top_p: float = 0.9
    repeat_penalty: float = 1.05
    num_ctx: int = 16384
    num_predict: int = 4096
    system_prompt: str = TACTICIAN_SYSTEM_PROMPT


@dataclass
class OllamaExportResult:
    """Paths and helper commands emitted by `export_ollama_bundle`."""

    out_dir: Path
    modelfile_path: Path
    helper_script_path: Path
    manifest_path: Path
    model_tag: str
    quantization: str
    modelfile_text: str
    commands: dict[str, str]

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema": "tactician-ollama-export-v1",
            "out_dir": str(self.out_dir),
            "modelfile_path": str(self.modelfile_path),
            "helper_script_path": str(self.helper_script_path),
            "manifest_path": str(self.manifest_path),
            "model_tag": self.model_tag,
            "quantization": self.quantization,
            "commands": dict(self.commands),
        }


def resolve_ollama_tag(config: OllamaExportConfig) -> str:
    if config.model_tag and str(config.model_tag).strip():
        return str(config.model_tag).strip()
    preset = resolve_model_preset(config.base_model)
    return str(preset.get("recommended_ollama_tag") or DEFAULT_OLLAMA_MODEL_TAG)


def render_ollama_modelfile(config: OllamaExportConfig) -> str:
    """Render an Ollama `Modelfile` string for the trained Digimon Tactician."""
    tag = resolve_ollama_tag(config)
    preset = resolve_model_preset(config.base_model)
    from_val = str(config.from_source).strip()
    if not from_val:
        raise ValueError("OllamaExportConfig.from_source must be non-empty")
    if int(config.num_ctx) < 2048:
        raise ValueError(f"num_ctx must be >= 2048 for tactical board + card text context, got {config.num_ctx}")
    if not (0.0 <= float(config.temperature) <= 2.0):
        raise ValueError(f"temperature must lie in [0.0, 2.0], got {config.temperature}")

    escaped_system = str(config.system_prompt).replace('"""', '\\"\\"\\"')
    lines = [
        f"# Digimon Tactician Ollama Modelfile ({tag})",
        f"# Base Model: {config.base_model} ({preset.get('license', 'Apache-2.0')})",
        f"# Target Quantization: {config.quantization} (~{preset.get('vram_q4_k_m_gb', 5.2)} GB VRAM on RTX 4070)",
        f"FROM {from_val}",
    ]
    if config.adapter_dir is not None:
        lines.append(f"ADAPTER {config.adapter_dir}")

    lines.extend(
        [
            "",
            f"PARAMETER temperature {float(config.temperature):.2f}",
            f"PARAMETER top_p {float(config.top_p):.2f}",
            f"PARAMETER repeat_penalty {float(config.repeat_penalty):.2f}",
            f"PARAMETER num_ctx {int(config.num_ctx)}",
            f"PARAMETER num_predict {int(config.num_predict)}",
            "",
            f'SYSTEM """{escaped_system}"""',
            "",
        ]
    )
    return "\n".join(lines)


def build_helper_commands(config: OllamaExportConfig, modelfile_path: Path) -> dict[str, str]:
    """Return copy-pasteable GGUF quantization, Ollama registration, and `puzzle_lab` commands."""
    tag = resolve_ollama_tag(config)
    quant = str(config.quantization or "Q4_K_M").upper()
    from_str = str(config.from_source)
    gguf_out = from_str if from_str.endswith(".gguf") else f"./{Path(from_str).name}-{quant.lower()}.gguf"
    f16_gguf = gguf_out.replace(f"-{quant.lower()}.gguf", "-f16.gguf")
    if f16_gguf == gguf_out:
        f16_gguf = "./digimon-tactician-f16.gguf"

    return {
        "convert_hf_to_gguf": (
            f"python convert_hf_to_gguf.py ./merged_tactician_hf --outfile {f16_gguf} --outtype f16"
        ),
        "llama_quantize": f"llama-quantize {f16_gguf} {gguf_out} {quant}",
        "ollama_create": f"ollama create {tag} -f {modelfile_path}",
        "puzzle_lab_check": (
            f"PYTHONPATH=code python -m tools.puzzle_lab check-providers "
            f"--provider ollama --scanner-model {tag} --author-model {tag} --ping"
        ),
        "puzzle_lab_synthesize": (
            f"PYTHONPATH=code python -m tools.puzzle_lab synthesize-archetype "
            f"--provider ollama --scanner-model {tag} --author-model {tag} --archetype Imperialdramon"
        ),
    }


def export_ollama_bundle(
    config: OllamaExportConfig,
    *,
    out_dir: Path,
    modelfile_name: str = "Modelfile",
) -> OllamaExportResult:
    """Write `Modelfile`, `deploy_ollama.sh`, and `export_manifest.json` inside `out_dir`."""
    dest = Path(out_dir)
    dest.mkdir(parents=True, exist_ok=True)

    modelfile_text = render_ollama_modelfile(config)
    modelfile_path = dest / modelfile_name
    modelfile_path.write_text(modelfile_text, encoding="utf-8")

    tag = resolve_ollama_tag(config)
    cmds = build_helper_commands(config, modelfile_path)

    preset = resolve_model_preset(config.base_model)
    script_lines = [
        "#!/usr/bin/env bash",
        "set -euo pipefail",
        f"# Local RTX 4070 (12 GB VRAM) Ollama Deployment Helper for {tag}",
        f"# Estimated VRAM ({config.quantization}): ~{preset.get('vram_q4_k_m_gb', 5.2)} GB",
        "",
        f"# Step 1 (on RunPod or local with llama.cpp): convert merged HF checkpoint & quantize to {config.quantization}",
        f"# {cmds['convert_hf_to_gguf']}",
        f"# {cmds['llama_quantize']}",
        "",
        "# Step 2: Register quantized GGUF in local Ollama",
        cmds["ollama_create"],
        "",
        "# Step 3: Verify endpoint & run Puzzle Lab with your local open-weight Tactician",
        f"# {cmds['puzzle_lab_check']}",
        f"# {cmds['puzzle_lab_synthesize']}",
        "",
    ]
    helper_script_path = dest / "deploy_ollama.sh"
    helper_script_path.write_text("\n".join(script_lines), encoding="utf-8")

    manifest_path = dest / "export_manifest.json"
    result = OllamaExportResult(
        out_dir=dest,
        modelfile_path=modelfile_path,
        helper_script_path=helper_script_path,
        manifest_path=manifest_path,
        model_tag=tag,
        quantization=str(config.quantization),
        modelfile_text=modelfile_text,
        commands=cmds,
    )
    manifest_payload = {
        **result.to_dict(),
        "base_model": config.base_model,
        "model_preset": preset,
        "parameters": {
            "temperature": float(config.temperature),
            "top_p": float(config.top_p),
            "repeat_penalty": float(config.repeat_penalty),
            "num_ctx": int(config.num_ctx),
            "num_predict": int(config.num_predict),
        },
    }
    manifest_path.write_text(json.dumps(manifest_payload, indent=2) + "\n", encoding="utf-8")
    return result
