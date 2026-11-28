# Digimon Tactician GRPO Runbook (`tools.tactician_grpo`)

Operator guide for training an open-weight, engine-grounded Digimon TCG tactical solver and puzzle composer on RunPod (`TRL` QLoRA SFT + Verifiable-Reward GRPO) and deploying it locally on a 12 GB RTX 4070 via `Ollama` (`Q4_K_M` GGUF) for `tools.puzzle_lab`.

---

## 1. Architecture & Licensing Boundary

`code/tools/tactician_grpo/` is a standalone dev-only package:

- **Open-Weight Only (Apache-2.0)**: Defaults to `Qwen/Qwen3-8B` (`Qwen/Qwen3-14B` supported via `--base-model`). Closed/proprietary model prefixes (`gpt-*`, `claude-*`, `gemini-*`) are rejected by `validate_train_config`.
- **Zero Heavy CI Imports**: `torch`, `transformers`, `peft`, `trl`, `datasets`, and `bitsandbytes` are lazily imported only when `train` executes without `--dry-run`. Dataset export, 5-stage reward scoring, `--dry-run` config validation, and Ollama `Modelfile` export run in milliseconds on CPU.
- **Production Isolation**: Production runtime surfaces (`code/server/`, `code/digimon_gym/`, `code/src-tauri/`) must never import `tools.tactician_grpo`.

| Model | License | RunPod Training Target | Local `Q4_K_M` GGUF | Co-Resides with PPO / Studio on 12 GB RTX 4070? | Recommended Ollama Tag |
|---|---|---|---|---|---|
| `Qwen/Qwen3-8B` *(default)* | Apache-2.0 | 1× RTX 4090 (24 GB) / A100 (40 GB) | `~5.2 GB` | **Yes** (~6.8 GB headroom for ONNX / PPO / Tauri Studio) | `digimon-tactician:8b` |
| `Qwen/Qwen3-14B` | Apache-2.0 | 1× A100 (40/80 GB) / A6000 (48 GB) | `~9.0 GB` | Standalone Puzzle Lab mode (pause GPU PPO worker) | `digimon-tactician:14b` |

---

## 2. Step 1 — Export SFT & GRPO Datasets

Scan `qa/puzzles/` (`studio-puzzle-v1`) and `qa/search-exams/` (`search-exam-v1`), enrich all cards in play/hand/trash/breeding with printed `effect_text`, `inherited_text`, `security_text`, `link_text`, `traits`, and `evo_costs` from `digimon_engine.CardDatabase` + `data/cards.json`, and attach concise keyword/timing excerpts from `docs/RULES_CONTEXT.md` (`docs/digimon-rules/`):

```bash
PYTHONPATH=code python -m tools.tactician_grpo export-dataset \
  --out-dir runs/tactician_grpo/datasets
```

Emits two JSONL files in `runs/tactician_grpo/datasets/`:

1. **`sft_train.jsonl`**: Conversational `messages` (`system`, `user`, `assistant` with `<think>...</think>` + JSON) covering both task modes:
   - **Task Mode 1 (`solve_position`)**: Given staged board + printed card texts + decoded legal actions, emit `tactician-solve-v1` JSON with the winning `target` line and refuted `avoid` trap line(s).
   - **Task Mode 2 (`compose_counter_puzzle`)**: Given a locked Defender Boss board + `[All Turns]` `refutation_triggers` + Challenger's admitted decklist/card texts, compose Challenger's mid-game zones and `target`/`avoid` lines as a complete `studio-puzzle-v1` artifact.
2. **`grpo_prompts.jsonl`**: TRL `GRPOTrainer` rows (`prompt`, `task_kind`, `ground_truth_meta` carrying `allowed_decks`, `defender_zones`, `refutation_triggers`, and `base_puzzle`).

---

## 3. Step 2 — 5-Stage Dense Verifiable Engine Reward

`DigimonVerifiableReward` evaluates completions directly in-process against `digimon_engine` (`RustDebugGame`, `RustHeadlessGame`, `studio_puzzles.select_refutation_action`, and `CardAdmissibility`), evaluating 8–16 GRPO rollouts in milliseconds with zero subprocess overhead (or optionally via `StdioMcpClient` with `--use-mcp`):

| Stage | Delta | Cumulative | Verification Check |
|---|---|---|---|
| **Stage 1 — Valid JSON & Schema** | `+0.15` | `0.15` | Strips `<think>...</think>` & fences; validates `studio-puzzle-v1` / `tactician-solve-v1` fields. |
| **Stage 2 — Card Admissibility & Purity** | `+0.15` | `0.30` | Every non-`TOKEN_*` card is admissible in `data/card_admissibility.json` and within Challenger/Defender deck copy counts. |
| **Stage 3 — Engine Board Legality** | `+0.20` | `0.50` | `RustDebugGame` + `skip_mulligan()` + `apply_scenario()` + legal `prefix_actions` + `validate()` succeed cleanly. |
| **Stage 4 — `target` Line Verified** | `+0.35` | `0.85` | Stepping `target` on `RustHeadlessGame` against `refutation_triggers` + `greedy_action` adversary interrupts achieves terminal win (or satisfies all `rl_goal.win_assertions` in `bounded_horizon` mode). |
| **Stage 5 — `avoid` Line Refuted** | `+0.15` | `1.00` | At least one non-prefix `avoid` line is legal at its root action and gets refuted / fails to satisfy the goal. |

Spot-check any puzzle or model completion locally:

```bash
PYTHONPATH=code python -m tools.tactician_grpo score-completion \
  --completion-file qa/puzzles/ex12-erlangmon-saneiketsu-invitation-trap.json
```

---

## 4. Step 3 — RunPod QLoRA SFT + GRPO Training

### 4.1 Dry-Run Validation (Local CPU / CI Preflight)

Validate hyperparameters, QLoRA settings (`lora_r=32`, `lora_alpha=64`, 4-bit NF4), JSONL schemas, and live reward scoring before spinning up a GPU pod:

```bash
PYTHONPATH=code python -m tools.tactician_grpo train --dry-run \
  --base-model Qwen/Qwen3-8B \
  --sft-dataset runs/tactician_grpo/datasets/sft_train.jsonl \
  --grpo-dataset runs/tactician_grpo/datasets/grpo_prompts.jsonl \
  --output-dir runs/tactician_grpo/qwen3_8b
```

### 4.2 RunPod GPU Execution

On a RunPod PyTorch 2.4+ pod (with compiled `digimon_engine` wheel installed):

```bash
pip install "torch>=2.4" "transformers>=4.51" "peft>=0.14" "trl>=0.15" "datasets>=3.0" "bitsandbytes>=0.45" accelerate

# Train Qwen/Qwen3-8B (default; ~5.2 GB Q4_K_M target for 12 GB RTX 4070 co-residency)
PYTHONPATH=code python -m tools.tactician_grpo train \
  --base-model Qwen/Qwen3-8B \
  --sft-dataset runs/tactician_grpo/datasets/sft_train.jsonl \
  --grpo-dataset runs/tactician_grpo/datasets/grpo_prompts.jsonl \
  --output-dir runs/tactician_grpo/qwen3_8b \
  --sft-epochs 2 \
  --grpo-max-steps 200 \
  --grpo-num-generations 8

# Optional heavier author model: Qwen/Qwen3-14B
PYTHONPATH=code python -m tools.tactician_grpo train \
  --base-model Qwen/Qwen3-14B \
  --sft-dataset runs/tactician_grpo/datasets/sft_train.jsonl \
  --grpo-dataset runs/tactician_grpo/datasets/grpo_prompts.jsonl \
  --output-dir runs/tactician_grpo/qwen3_14b
```

Checkpoints are written to `<output-dir>/sft_adapter` and `<output-dir>/grpo_adapter`.

---

## 5. Step 4 — Merge, Quantize (`Q4_K_M`), and Deploy on Local RTX 4070 `Ollama`

### 5.1 Merge QLoRA Adapter & Quantize to GGUF `Q4_K_M`

On the RunPod pod (before terminating) or locally:

```bash
python - <<'PY'
import torch
from peft import PeftModel
from transformers import AutoModelForCausalLM, AutoTokenizer

base_id = "Qwen/Qwen3-8B"
adapter_dir = "runs/tactician_grpo/qwen3_8b/grpo_adapter"
merged_dir = "runs/tactician_grpo/qwen3_8b/merged_hf"

tok = AutoTokenizer.from_pretrained(base_id, trust_remote_code=True)
base = AutoModelForCausalLM.from_pretrained(base_id, torch_dtype=torch.float16, device_map="cpu", trust_remote_code=True)
merged = PeftModel.from_pretrained(base, adapter_dir).merge_and_unload()
merged.save_pretrained(merged_dir)
tok.save_pretrained(merged_dir)
PY

# Convert merged HF weights to FP16 GGUF and quantize to Q4_K_M (~5.2 GB) using llama.cpp
python convert_hf_to_gguf.py runs/tactician_grpo/qwen3_8b/merged_hf \
  --outfile runs/tactician_grpo/ollama/digimon-tactician-qwen3-8b-f16.gguf \
  --outtype f16

llama-quantize \
  runs/tactician_grpo/ollama/digimon-tactician-qwen3-8b-f16.gguf \
  runs/tactician_grpo/ollama/digimon-tactician-qwen3-8b-q4_k_m.gguf \
  Q4_K_M
```

### 5.2 Generate Ollama `Modelfile` & Register Locally

```bash
PYTHONPATH=code python -m tools.tactician_grpo export-ollama \
  --from-source ./digimon-tactician-qwen3-8b-q4_k_m.gguf \
  --base-model Qwen/Qwen3-8B \
  --model-tag digimon-tactician:8b \
  --out-dir runs/tactician_grpo/ollama

ollama create digimon-tactician:8b -f runs/tactician_grpo/ollama/Modelfile
```

### 5.3 Plug Into `tools.puzzle_lab`

Verify connectivity and run autonomous puzzle synthesis / replay mining against your local RTX 4070 `Ollama` server:

```bash
# 1. Health check
PYTHONPATH=code python -m tools.puzzle_lab check-providers \
  --provider ollama \
  --scanner-model digimon-tactician:8b \
  --author-model digimon-tactician:8b \
  --ping

# 2. Synthesize verified Studio puzzles with local open-weight Tactician
PYTHONPATH=code python -m tools.puzzle_lab synthesize-archetype \
  --provider ollama \
  --scanner-model digimon-tactician:8b \
  --author-model digimon-tactician:8b \
  --archetype "Imperialdramon"
```
