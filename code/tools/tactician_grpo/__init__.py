"""RunPod SFT + Verifiable-Reward GRPO -> Local RTX 4070 `Qwen3-8B` Ollama Tactician Pipeline.

Standalone dev-only package (`tools.tactician_grpo`):
- `dataset_exporter`: Enriches `qa/puzzles/` & `qa/search-exams/` with printed card
  texts (`CardDatabase` + `data/cards.json`) and rules excerpts (`docs/RULES_CONTEXT.md`),
  emitting `sft_train.jsonl` and `grpo_prompts.jsonl`.
- `verifiable_reward`: 5-stage dense engine reward (`DigimonVerifiableReward`)
  evaluated in-process on `RustDebugGame` / `RustHeadlessGame` (plus optional
  `StdioMcpClient` backend).
- `train_grpo`: 2-stage QLoRA `SFTTrainer` -> `GRPOTrainer` pipeline defaulting to
  Apache-2.0 `Qwen/Qwen3-8B` (`Qwen/Qwen3-14B` supported), with lazy GPU imports
  and zero-GPU `--dry-run` validation.
- `export_ollama`: Generates `Modelfile` (`Q4_K_M`, `num_ctx 16384`) and quantization
  commands for local 12 GB RTX 4070 deployment alongside `tools.puzzle_lab --provider ollama`.

Service Boundary:
Production surfaces (`code/server/`, `code/digimon_gym/`, `code/src-tauri/`) must
never import `tools.tactician_grpo`.
"""

from __future__ import annotations

from .dataset_exporter import (
    TACTICIAN_SYSTEM_PROMPT,
    DatasetExportSummary,
    enrich_card_catalog_entries,
    export_tactician_datasets,
    extract_rules_excerpts,
)
from .export_ollama import (
    DEFAULT_OLLAMA_MODEL_TAG,
    OllamaExportConfig,
    OllamaExportResult,
    export_ollama_bundle,
    render_ollama_modelfile,
)
from .train_grpo import (
    DEFAULT_BASE_MODEL,
    MODEL_PRESETS,
    SUPPORTED_BASE_MODELS,
    TacticianTrainConfig,
    resolve_model_preset,
    run_sft_and_grpo,
    validate_train_config,
)
from .verifiable_reward import (
    STAGE1_WEIGHT,
    STAGE2_WEIGHT,
    STAGE3_WEIGHT,
    STAGE4_WEIGHT,
    STAGE5_WEIGHT,
    TACTICIAN_SOLVE_SCHEMA,
    TASK_COMPOSE_COUNTER_PUZZLE,
    TASK_SOLVE_POSITION,
    DigimonVerifiableReward,
    RewardBreakdown,
    parse_completion_json,
    score_single_completion,
    strip_thinking_blocks,
)

__version__ = "0.1.0"

__all__ = [
    "DEFAULT_BASE_MODEL",
    "DEFAULT_OLLAMA_MODEL_TAG",
    "MODEL_PRESETS",
    "STAGE1_WEIGHT",
    "STAGE2_WEIGHT",
    "STAGE3_WEIGHT",
    "STAGE4_WEIGHT",
    "STAGE5_WEIGHT",
    "SUPPORTED_BASE_MODELS",
    "TACTICIAN_SOLVE_SCHEMA",
    "TACTICIAN_SYSTEM_PROMPT",
    "TASK_COMPOSE_COUNTER_PUZZLE",
    "TASK_SOLVE_POSITION",
    "DatasetExportSummary",
    "DigimonVerifiableReward",
    "OllamaExportConfig",
    "OllamaExportResult",
    "RewardBreakdown",
    "TacticianTrainConfig",
    "enrich_card_catalog_entries",
    "export_ollama_bundle",
    "export_tactician_datasets",
    "extract_rules_excerpts",
    "parse_completion_json",
    "render_ollama_modelfile",
    "resolve_model_preset",
    "run_sft_and_grpo",
    "score_single_completion",
    "strip_thinking_blocks",
    "validate_train_config",
]
