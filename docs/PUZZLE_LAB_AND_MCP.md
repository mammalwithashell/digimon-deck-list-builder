# ModelPilot + LLM Tactical Puzzle Lab & Dual-MCP Guide

Operator reference for pairing trained ONNX (`ModelPilot`) and SB3 `.zip` checkpoints with interactive MCP chat sessions, local open-weight LLMs (RTX 4070 `Ollama` / `llama.cpp`), cloud open-weight GPUs (RunPod `vLLM` / `SGLang`), and dev-only closed frontier models (`Gemini 4 Neon` / `Gemini 4 Hydrogen`).

Design specification: [`docs/superpowers/specs/2026-11-27-model-pilot-llm-puzzle-lab-design.md`](superpowers/specs/2026-11-27-model-pilot-llm-puzzle-lab-design.md).

---

## 1. Architecture Overview

The workflow splits cleanly across two MCP servers and two standalone CLI tools, all sharing canonical **`studio-puzzle-v1`** files under `qa/puzzles/` (which open directly in the `/studio` UI):

1. **`digimon-engine-mcp` (Rust — `.onnx` Fast Path, MCTS, Stack Text & Adversary Verification)**:
   - Caches `.onnx` MLP and LSTM (+ `.value.onnx` sidecar) models in memory via `ModelRegistry`.
   - Exposes `inspect_position`, `score_line`, `inspect_stack` (top `effect_text` + under-card `inherited_text` only + `link_text` only), `load_puzzle`, `save_puzzle`, and `verify_puzzle`.
   - Enforces `mode: "play"` (`Perspective` seat-locking + auto-stepping `opponent_driver`) vs. `mode: "author"` (omniscient staging & export).
2. **`digimon-training-mcp` (Python — `.zip` White-Box PyTorch Diagnostics)**:
   - Strictly read-only over `runs/` and `models/`.
   - Exposes `card_embedding_neighbors`, `position_saliency`, `plasticity_probe` (2-second tensor-blindness detector), and cached `export_checkpoint_onnx` (converting any `step_*.zip` into a cached `.onnx` for `digimon-engine-mcp`).
3. **`code/tools/emit_puzzle_shards.py` + `studio_puzzles.py` (Supervised Shards & Adaptive RL Curriculum)**:
   - Permutes solver hand/field slot indices + inert distractors (`permute_staged_puzzle`), emits `selfplay-shards-v1` (+ contrastive `avoid` edges) for `azero_trainer.py` / `azero_lstm_distill.py`, and schedules puzzle episode resets via `AdaptivePuzzleScheduler` in `pilot_training.py`.
4. **`code/tools/puzzle_lab/` (Dev-Only Multi-Provider LLM CLI)**:
   - Automates `mine-replays`, `synthesize-archetype`, and `compose-boss-matrix` across `Ollama`, `vLLM`, or dev-gated `Gemini 4 Hydrogen` + `Gemini 4 Neon`.

---

## 2. Interactive MCP Setup (`.mcp.json`)

Build the Rust MCP binary and install the Python training MCP with `[whitebox]` dependencies (in a dedicated venv per `requirements-mcp.txt`):

```bash
cargo build -p digimon-engine-mcp
pip install -r requirements-training.txt -r requirements-mcp.txt
pip install --no-deps -e . -e code/digimon-training-mcp
maturin develop --manifest-path code/digimon-engine-py/Cargo.toml
```

Register both servers in `.mcp.json`:

```json
{
  "mcpServers": {
    "digimon-engine-mcp": {
      "type": "stdio",
      "command": "target/debug/digimon-engine-mcp",
      "args": ["--pool", "implemented"]
    },
    "digimon-training-mcp": {
      "type": "stdio",
      "command": "python",
      "args": ["-m", "digimon_training_mcp", "--runs-dir", "./runs", "--models-dir", "./models"]
    }
  }
}
```

### Key Interactive MCP Recipes

#### Recipe A — Play Fairly Against Your Trained Weights in Chat
1. Export or locate an `.onnx` checkpoint (or call `digimon-training-mcp`'s `export_checkpoint_onnx` on any `step_*.zip`).
2. Call `load_model({ "model_id": "gen1", "onnx_path": "models/gen1/policy.onnx" })`.
3. Call `load_puzzle({ "puzzle_path": "qa/puzzles/ex12-erlangmon-saneiketsu-invitation-trap.json", "mode": "play", "opponent_driver": "mcts:gen1:128" })` (or `new_game_from_decks` with `mode: "play", seat: 0`).
   - `view="god"`, opponent hands, face-down security, and `save_puzzle`/`score_line` are automatically blocked so the chat LLM cannot peek at hidden information.
   - Use `inspect_stack` to read exact composite stack effects and `step` / `digivolve` / `attack` to play; the engine auto-steps opponent turns and `[All Turns]` interrupts via `opponent_driver`.

#### Recipe B — Diagnose a Blunder & Author a Verified Puzzle
1. Call `card_embedding_neighbors({ "checkpoint_path": "models/gen1/final.zip", "card_ids": ["EX12-034", "EX12-071"] })` to verify the checkpoint's learned embeddings aren't starved.
2. Call `load_puzzle(..., mode="author")` -> `score_line({ "game_id": "...", "model_id": "gen1", "actions": [0, 101], "refutation_triggers": [...] })` to inspect per-step value shifts and opponent interrupt chains.
3. Call `position_saliency` on `digimon-training-mcp` to see which board slots/sections drive `logit(target) - logit(avoid)` and strip ~0-saliency clutter cards.
4. Call `verify_puzzle` (proving `target` wins against adversary interrupts while `avoid` is refuted) + `plasticity_probe` (confirming 10 in-memory gradient steps separate `target` from `avoid`, ruling out `standard_lite_v2` tensor blindness).
5. Call `save_puzzle` to write `qa/puzzles/<folder>/<id>.json` (and optional `qa/search-exams/<id>.json`).

---

## 3. Adversary `[All Turns]` Interrupt Grading (`search_exam`)

Both `verify_puzzle` (MCP) and the Rust `search_exam` runner treat any step where `decision_player != solver` as an adversary node, supporting `refutation_triggers` and bounded adversary search (`--adversary-sims`):

```bash
# Grade search expectations + verify target/avoid lines against adversary interrupts
cargo run --release -p digimon-engine --example search_exam -- \
  qa/search-exams/ex12-erlangmon-saneiketsu-invitation-trap.json \
  --model models/gen1/policy.onnx \
  --seeds 8 \
  --verify-adversary \
  --adversary-sims 64
```

---

## 4. Warm-Starting & Training with Puzzles

### 4.1 Supervised Puzzle Shards (`emit_puzzle_shards.py` + `azero_*`)

Generate slot/hand-permuted `selfplay-shards-v1` trajectories (with contrastive `avoid` edges) and mix them alongside regular self-play shards:

```bash
# 1. Emit 32 verified slot/hand permutations per puzzle
python code/tools/emit_puzzle_shards.py \
  --inputs qa/puzzles qa/search-exams/ex13-imperialdramon-dna-paildramon-jamming-lethal.json \
  --out runs/puzzle_shards/ex13_drills \
  --permutations-per-puzzle 32 \
  --observation-profile standard_lite_v2

# 2a. Fine-tune MLP policy+value with contrastive avoid-margin loss
python -m digimon_gym.agents.azero_trainer \
  --init-from models/generalist_base/final.zip \
  --shards runs/selfplay/gen0 runs/puzzle_shards/ex13_drills \
  --avoid-margin-coef 0.2 \
  --avoid-margin 1.0 \
  --out models/generalist_warm/final.zip \
  --export-onnx models/generalist_warm/policy.onnx

# 2b. Or distill into recurrent LSTM policy+value
python -m digimon_gym.agents.azero_lstm_distill \
  --init-from models/lstm_base/final.zip \
  --shards runs/selfplay/gen0 runs/puzzle_shards/ex13_drills \
  --avoid-margin-coef 0.2 \
  --out models/lstm_warm/final.zip
```

### 4.2 RL Episode Curriculum Resets (`pilot_training.py`)

Start a fraction of PPO episodes directly on slot-permuted Studio puzzles, dynamically weighted by `AdaptivePuzzleScheduler` (prioritizing unsolved puzzles while keeping a maintenance floor on solved ones) and firing `refutation_triggers` inside `OpponentWrapper`:

```bash
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training \
  --init-from models/generalist_warm/final.zip \
  --generalist \
  --studio-puzzles-dir qa/puzzles \
  --puzzle-prob 0.15 \
  --puzzle-permute \
  --tensor-profile standard_lite_v2 \
  --timesteps 500000
```

Always validate candidate checkpoints on full-game seat-balanced anchored evaluation (`code/tools/anchored_eval_cli.py`) alongside the `search_exam` suite.

---

## 5. Multi-Provider LLM Puzzle Lab CLI (`tools.puzzle_lab`)

`tools.puzzle_lab` is a **dev-only** CLI (strictly isolated from `code/server/`, `code/digimon_gym/`, and `code/src-tauri/`). Closed-weight providers (`gemini`) raise `PermissionError` unless explicitly authorized via `DIGIMON_ALLOW_CLOSED_LLM=1` or `--allow-closed-models`.

### 5.1 Provider Configurations

```bash
# A. Local RTX 4070 via Ollama (Open-Weight, $0 API cost)
PYTHONPATH=code python -m tools.puzzle_lab check-providers \
  --provider ollama \
  --scanner-model qwen3:8b \
  --author-model qwen3:14b \
  --ping

# B. RunPod vLLM / SGLang Endpoint (Heavy Open-Weight Reasoner)
PYTHONPATH=code python -m tools.puzzle_lab check-providers \
  --provider vllm \
  --base-url https://<pod-id>-8000.proxy.runpod.net/v1 \
  --scanner-model Qwen/Qwen3-14B \
  --author-model Qwen/Qwen3-32B \
  --ping

# C. Dev-Only Closed Frontier Models (Gemini 4 Hydrogen Scanner + Gemini 4 Neon Author)
DIGIMON_ALLOW_CLOSED_LLM=1 GEMINI_API_KEY=... \
PYTHONPATH=code python -m tools.puzzle_lab check-providers \
  --provider gemini \
  --allow-closed-models \
  --scanner-model gemini-4-hydrogen \
  --author-model gemini-4-neon \
  --pin-rules-cache \
  --ping
```

*(You can also mix providers—for example, `--scanner-provider ollama --scanner-model qwen3:8b` locally on your RTX 4070 paired with `--author-provider gemini --author-model gemini-4-neon --allow-closed-models` for hard multi-trigger authoring.)*

### 5.2 Subcommands

#### 1. Mine Replays (`mine-replays`)
Pre-filters native recordings + `.winprob.json` sidecars + `CardEvalCollector` reports without spending LLM tokens, triages candidates with `SCANNER`, and runs `AUTHOR` with MCP verification + optional `plasticity_probe`:

```bash
PYTHONPATH=code python -m tools.puzzle_lab mine-replays \
  runs/gen1/recordings \
  --winprob runs/gen1/winprob \
  --checkpoint-zip models/gen1/final.zip \
  --onnx-model models/gen1/policy.onnx \
  --require-plasticity \
  --export-search-exams \
  --provider ollama
```

#### 2. Synthesize Archetype Drills (`synthesize-archetype`)
Reads an archetype's admitted decklist (`data/deck_library.json`) + printed card text (`data/cards.json`), checks `card_embedding_neighbors` when `--checkpoint-zip` is given, and authors verified offensive combos and `[All Turns]` interrupt traps:

```bash
PYTHONPATH=code python -m tools.puzzle_lab synthesize-archetype \
  --archetype "Examon" \
  --opponent-archetype "Shambala" \
  --checkpoint-zip models/gen1/final.zip \
  --onnx-model models/gen1/policy.onnx \
  --count 4 \
  --require-plasticity \
  --export-search-exams \
  --provider vllm --base-url https://<pod-id>-8000.proxy.runpod.net/v1
```

#### 3. Parameterized Boss-Board × Archetype Counter-Matrix (`compose-boss-matrix`)
Sweeps a reusable Defender Boss-Board Template (such as the built-in `ex12-erlangmon-invitation` preset) across security/memory knobs and Challenger archetypes, automatically retrying when Erlangmon's interrupt refutes an attempt and grading `--eval-models` across the matrix:

```bash
DIGIMON_ALLOW_CLOSED_LLM=1 GEMINI_API_KEY=... \
PYTHONPATH=code python -m tools.puzzle_lab compose-boss-matrix \
  --boss-template ex12-erlangmon-invitation \
  --training-pool-snapshot data/training_pools/ex13-superwide-20261127/train_snapshot.json \
  --archetypes "Imperialdramon (UG),Examon,Royal Knights" \
  --security-counts 1,2 \
  --memories 2,4 \
  --eval-models models/gen1/policy.onnx \
  --export-search-exams \
  --provider gemini --allow-closed-models --pin-rules-cache
```

#### 4. Guided Play-In Compiler & Symbolic DCGO YAML Export (`compile-play-in` / MCP `compile_play_in`)
Compiles a mid-game staged `studio-puzzle-v1` board into a 100% legal Turn-1 stacked-deck + `prefix_actions` setup via Rust goal-directed beam search (`digimon_engine::playin`) with optional `Role.AUTHOR` LLM macro-turn fallback, preserving `KnowledgeLedger` (`closed-list` belief compatibility), pre-warming LSTM `(h, c)` hidden states, remapping solver lines, and exporting symbolic `dcgo-harness` YAML:

```bash
PYTHONPATH=code python -m tools.puzzle_lab compile-play-in \
  qa/puzzles/ex12-erlangmon-saneiketsu-invitation-trap.json \
  --beam-width 32 --max-turns 5 \
  --out-puzzle runs/playin/erlangmon_replay_prefix.json \
  --out-dcgo-yaml runs/playin/erlangmon_exam.yaml
```

*(Pass `--strict-zones` when exact `hand` and `trash` multisets are required instead of default functional equivalence.)*

#### 5. Observation-Tensor Completeness & Card-Fidelity Fuzzer (`fuzz-tensor` & `audit-set`)
Strictly read-only over active tensor contracts (`standard_lite_v2` / `standard_lite_deck_v2`):
- `fuzz-tensor` runs deterministic zero-LLM state-pair fuzzing across 5 core rule families (`opt_spent_vs_unspent`, `delay_armed_vs_placing_turn`, `modifier_active_vs_absent`, `facedown_vs_faceup_source`, `linked_vs_unlinked`), attributing `|obs_A - obs_B|` across `get_observation_layout(profile)` sections and optionally running `plasticity_probe`.
- `audit-set` prompts `Role.AUTHOR` with printed card text to probe `GameEvent` fidelity on implemented cards in a set:

```bash
# Zero-LLM tensor completeness fuzzing + white-box plasticity check
PYTHONPATH=code python -m tools.puzzle_lab fuzz-tensor \
  --profile standard_lite_v2 \
  --checkpoint-zip models/gen1/final.zip \
  --json-out runs/fuzz/tensor_fuzz.json \
  --markdown-out runs/fuzz/tensor_fuzz.md

# Read-only LLM card-set fidelity audit
PYTHONPATH=code python -m tools.puzzle_lab audit-set \
  --set EX12 --max-cards 10 \
  --provider ollama --author-model digimon-tactician:8b \
  --out-dir runs/fuzz/ex12_audit
```

#### 6. Air-Gapped Auto-Curriculum Sidecar (`auto-curriculum`)
Reads an `anchored_eval` summary or Specialist League `gates/gen<g>/*.json` directory (`specialist-gate-v1`), selects bottom-$K$ matchups (`--win-rate-threshold 0.45`), harvests drills, enforces the Mandatory Dual Gate (`verify_puzzle` + `plasticity_probe`), quarantines `observation_blindness` cases to `tensor_blindness_quarantine.json`, and emits a SHA256-stamped `selfplay-shards-v1` bundle (`curriculum_manifest.json`) for `--extra-shards` (`<= 10%` batch share):

```bash
PYTHONPATH=code python -m tools.puzzle_lab auto-curriculum \
  runs/league/gates/gen1 \
  --recordings runs/league/recordings \
  --checkpoint-zip models/gen1/final.zip \
  --out-dir runs/league/curriculum_gen1 \
  --provider ollama --author-model digimon-tactician:8b
```

---

## 6. Imperfect-Information Belief Puzzles & Mid-Prompt Critic Consistency

### 6.1 Belief Puzzles (`verify_belief_puzzle` & `--belief-worlds`)
Evaluate probabilistic hand-trap discipline (`closed-list` PIMC conditioned on `KnowledgeLedger` + `WeightedDeckPrior` vs. omniscient `open-list`) and emit multi-world Bayes-averaged root targets $(\pi_{\text{belief}}, \bar{z} \in [-1, +1])$:

```bash
# Grade closed-list PIMC + report Open-vs-Closed Information Gap
cargo run --release -p digimon-engine --example search_exam -- \
  qa/search-exams/ex13-belief-hand-trap-discipline.json \
  --model models/gen1/policy.onnx \
  --seeds 4 --belief-gap

# Emit multi-world belief shards
python code/tools/emit_puzzle_shards.py \
  --inputs qa/puzzles/ex13-belief-hand-trap-discipline.json \
  --out runs/puzzle_shards/belief_drills \
  --belief-worlds 16
```

### 6.2 Forced-Selection Value Rows, Critic Consistency & `--extra-shards` Cap
Cure intermediate `PendingSelection` value-head cliffs (`<Training>`, DNA material selection, `<Delay>`) without misaligning LSTM step clocks:
- `emit_puzzle_shards.py` (default on) and `digimon-engine-cli selfplay --include-forced-selection-value` emit value-only rows (`policy_weight = 0.0`, `forced_selection.npy`) plus `chain_anchor_idx.npy` across non-RNG single-mover resolution chains.
- `azero_trainer.py`, `azero_lstm_distill.py` (which filters `forced_selection == 1` out by default unless `--include-forced-selection` is passed), `run_selfplay_generation.py`, and `specialist_league.py` support `--consistency-coef 0.25` and `--extra-shards` capped at `--max-extra-shard-share 0.10`:

```bash
python -m digimon_gym.agents.azero_trainer \
  --init-from models/generalist_base/final.zip \
  --shards runs/selfplay/gen1 \
  --extra-shards runs/league/curriculum_gen1/extra_shards runs/puzzle_shards/belief_drills \
  --max-extra-shard-share 0.10 \
  --consistency-coef 0.25 \
  --avoid-margin-coef 0.2 \
  --out models/generalist_gen2/final.zip
```

---

## 7. Local RTX 4070 `Qwen3-8B` Digimon Tactician (`tools.tactician_grpo`)

Distill verified Digimon puzzle-solving and counter-staging reasoning into an Apache-2.0 `Qwen/Qwen3-8B` model (`~5.2 GB` `Q4_K_M` GGUF co-residing cleanly in 12 GB VRAM on an RTX 4070 alongside PPO/Studio) using RunPod `TRL` `SFTTrainer` + 5-stage verifiable-reward `GRPOTrainer`. See [`docs/TACTICIAN_GRPO_RUNBOOK.md`](TACTICIAN_GRPO_RUNBOOK.md) for the end-to-end RunPod -> local `Ollama` deployment runbook.

