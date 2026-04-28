# AGENTS.md - Agent Engineering Guide

Refer to `CLAUDE.md` for repository-wide architecture, service boundaries, and working rules.
Refer to `docs/RULES_CONTEXT.md` for rule implementation details.
Refer to `docs/TRAINING_RUNBOOK.md` for DB-backed training pipeline operations.
Refer to `docs/TENSOR_SPEC.md` and `docs/ACTION_SPEC.md` before changing observation or action contracts.

## Scope

This document describes the RL/deckbuilding agent stack in the current Rust-pivot repository. It focuses on the stable contracts that agent code depends on:

- Architect: deck optimization by card swaps.
- Pilot: battle play inside the headless engine.
- Training wrappers, gauntlets, model export, and DB-backed orchestration.

The no-approximations policy from `CLAUDE.md` applies to agent work: every legal choice must be surfaced through the engine action/pending-selection contracts so RL agents can learn it. Do not add card-effect stubs, auto-selections, or hidden UI-only decisions.

## Source Priority for Rules and Card Behavior

When deciding what a card, keyword, timing window, or selection should do, use sources in this order:

1. Printed card text in `data/cards.json` (`effect_text`, `inherited_text`, `security_text`).
2. `docs/RULES_CONTEXT.md`, plus the canonical rules PDF when needed.
3. Fandom wiki pages for card-specific ruling notes and errata context.
4. DCGO C# source in `DCGO/` as a behavioral implementation reference and tiebreaker.

DCGO is useful for detailed flow, but it is not the authority on optionality or mandatory behavior. Printed text and rules win.

## Engine Status

The project is migrating to the Rust engine as the source of truth:

- Target engine: `code/digimon-engine/`.
- Python bindings: `code/digimon-engine-py/`, exposed as `digimon_engine`.
- RL environment: `code/digimon_gym/digimon_gym.py`.
- Legacy Python engine: `code/engine_py_legacy/engine/`, retained as transitional reference/fallback only.

`DigimonEnv` chooses the runner with `DIGIMON_BACKEND`:

- `DIGIMON_BACKEND=rust`: use `RustHeadlessGame` from the PyO3 wheel.
- unset or other value: legacy Python `HeadlessGame` fallback where still explicitly wired.

New production behavior should target Rust first. Before editing engine behavior, check `docs/RUST_PYTHON_PARITY.md` for known divergences, and keep Rust/Python parity notes accurate while the migration is in progress.

---

# 1. Agent Tracks

The project has two active agent tracks:

1. Architect: deck optimizer trained to recommend card swaps against a target meta.
2. Pilot: battle agent trained/evaluated inside the headless engine.

Both are standalone training surfaces under `code/digimon_gym/agents/` and must not depend on FastAPI, auth, or database code.

---

# 2. Architect (Deck Builder Agent)

Goal: optimize a starting deck by proposing legal card swaps that improve weighted win rate against meta opponents.

## 2.1 Implemented Modules

- `architect_agent.py`: pure PyTorch `ArchitectDQN` with Double DQN, epsilon-greedy masked action selection, and prioritized experience replay.
- `architect_env.py`: Gymnasium `DeckBuildingEnv` for finite-horizon deck swap optimization.
- `architect_pool.py`: `CandidatePool`, swap action encoding, action masks, constraints, restricted-list handling, and candidate filtering.
- `architect_simulator.py`: batch win-rate evaluator for candidate decks using greedy/random/ONNX/SB3 pilot policies.
- `architect_optimizer.py`: end-to-end `MetaOptimizer` that builds opponents, trains the DQN, checkpoints, and saves optimized deck artifacts.
- `architect_training.py`: CLI entrypoint.
- `architect_cotraining.py`, `architect_explain.py`: supporting co-training/explanation utilities.

## 2.2 Architect MDP

State:

- Normalized deck vector over the candidate pool.
- Normalized meta-opponent weights.
- Step counter fraction.

Action:

- `0`: no-op, ending the episode early.
- `1..N`: encoded `(remove_card, add_card)` swap over the candidate pool.
- Total action size is `n_candidates * n_candidates + 1`.

Reward:

- Exponential win-rate delta: `exp(b * new_wr) - exp(b * old_wr)`.
- The default reward amplification constant is `10.0`.

Episode:

- Starts from a base deck.
- Separates Digi-Eggs from main deck; eggs are fixed and excluded from swap candidates.
- Applies up to `max_swaps` card swaps.
- Evaluates each candidate deck through `DeckSimulator`.

## 2.3 Candidate Pool Rules

`CandidatePool` keeps the action space tractable:

- Builds candidates from archetype decklists in `data/deck_library.json`, unless a custom pool is supplied.
- Always includes cards already in the base deck.
- Filters out Digi-Eggs from swap candidates.
- Filters to cards reported by `digimon_engine.load_implemented_card_ids()`.
- Supports extra cards, locked counts, min/max counts, and optional restricted-list enforcement.
- Uses stable sorted candidate ordering for reproducibility.

Action masks enforce:

- No-op is always valid.
- A card can be removed only if the current count is above its minimum.
- A card can be added only if the current count is below its maximum.
- A card cannot be swapped with itself.

## 2.4 Deck Simulator

`DeckSimulator` computes weighted win rate for an evaluated deck:

- Supports pilot policy names `"greedy"` and `"random"`.
- Supports `.onnx` policies through `digimon_gym.inference.onnx_policy`.
- Supports `.zip` SB3 models (`MaskablePPO` or `MaskableRecurrentPPO`).
- Alternates play/draw sides across games to reduce first-player bias.
- Counts simulation crashes as draws so card-script gaps do not poison training runs.
- Can cache matchup results by order-independent deck hash.
- Can evaluate in-process or through `ProcessPoolExecutor`.

LSTM policies must call/reset their recurrent state at episode boundaries.

## 2.5 Architect CLI

Run from repo root:

```bash
python -m digimon_gym.agents.architect_training --archetype "Medusamon"
python -m digimon_gym.agents.architect_training --archetype "Medusamon" --meta local_meta.json --pilot models/mlp_agent.onnx --episodes 500
python -m digimon_gym.agents.architect_training --archetype "Medusamon" --extra-cards "BT24-001,EX10-036" --workers 4 --seed 42
```

Artifacts are saved under `architect_runs/` by default.

---

# 3. Pilot (Battle Agent)

Goal: play Digimon matches inside the headless engine to generate win/loss and policy quality signals.

## 3.1 Implemented Pilot Types

### MaskablePPO (MLP baseline)

- Feed-forward policy from `sb3_contrib.MaskablePPO` with `"MlpPolicy"`.
- Uses `ActionMasker` so illegal actions are never sampled.
- Training entrypoint: `code/digimon_gym/agents/pilot_training.py`.
- Observation shape comes from `digimon_engine.TENSOR_SIZE` and is currently `1375`.
- Action space comes from `digimon_engine.ACTION_SPACE_SIZE` and is currently `2168`.

### MaskableRecurrentPPO (Custom LSTM)

Custom implementation in `code/digimon_gym/agents/maskable_recurrent/`.

Motivation: SB3's `RecurrentPPO` and `MaskablePPO` are separate algorithms with no official combined implementation. This module merges recurrent state handling with invalid-action masking.

Architecture:

- `MaskableMlpLstmPolicy` (`policies.py`)
  - Extends `RecurrentActorCriticPolicy`.
  - Replaces the categorical action distribution with a maskable distribution.
  - Supports separate actor/critic LSTMs.
  - Accepts `action_masks` in `forward()`, `evaluate_actions()`, `get_distribution()`, `_predict()`, and `predict()`.
- `MaskableRecurrentRolloutBuffer` (`buffers.py`)
  - Extends `RecurrentRolloutBuffer`.
  - Stores action masks alongside recurrent state and episode-start boundaries.
  - Pads masks for recurrent minibatches; padded timesteps are excluded from loss.
- `MaskableRecurrentPPO` (`maskable_recurrent_ppo.py`)
  - Extends `RecurrentPPO`.
  - Collects action masks at rollout time.
  - Passes masks through policy evaluation during training.

Default LSTM settings from `pilot_training.py`:

- `lstm_hidden_size`: `256`.
- `n_lstm_layers`: `1`.
- `enable_critic_lstm`: `True`.
- `net_arch`: `dict(pi=[64], vf=[64])`.
- `batch_size`: forced to `n_steps` for recurrent training.

Inference rule:

- Thread `(state, episode_start)` or `(h, c)` state across steps in an episode.
- Reset state to `None` and `episode_start=True` at episode boundaries.
- The same reset rule applies to ONNX LSTM policies.

### Greedy Policy (Heuristic)

Location: `digimon_gym.digimon_gym.greedy_policy()`.

Used as the default fast baseline and common training opponent. Priority logic:

- Mulligan: mulligan if no level-3 Digimon in hand, otherwise keep.
- Breeding: hatch, then move, then pass.
- Main: keep-turn digivolve, then attack, then play, then pass.
- Attack scoring: lethal first, favorable DP trades next.
- Digivolve scoring: prefers digivolutions with cost not exceeding relative memory.
- `TRASH_CARD` selection: discard lowest-value card first.
- Other phases: pass or first valid action.

### Random Policy

`pilot_training.random_policy()` samples uniformly from valid actions. If no valid action exists, it falls back to `ACTION_PASS_TURN`.

## 3.2 Gym Environment Contract

Primary env: `DigimonEnv` in `code/digimon_gym/digimon_gym.py`.

API:

- `reset(seed, options)` returns `(obs, info)`.
- `options` supports per-episode `deck1` and `deck2` overrides.
- `step(action)` returns `(obs, reward, terminated, truncated, info)`.
- `action_mask()` returns an `int8` mask of shape `(ACTION_SPACE_SIZE,)`.
- `get_action_mask()` returns a boolean mask alias for compatibility.

Observation/action spaces:

- Observation: `Box(shape=(TENSOR_SIZE,), low=-10.0, high=20001.0, dtype=float32)`.
- Current `TENSOR_SIZE`: `1375`.
- Action: `Discrete(ACTION_SPACE_SIZE)`.
- Current `ACTION_SPACE_SIZE`: `2168`.

Mask delivery:

- `info["action_mask"]` is included on `reset()` and `step()`.
- `ActionMasker` calls through to `env.action_mask()`/`env.get_action_mask()` through the wrapper chain.
- Engine masks use `1.0` for legal, `0.0` for illegal; env masks threshold at `> 0.5`.

Reward shaping:

- Terminal win: `+1.0`.
- Terminal loss: `-1.0`.
- Draw: `0.0`.
- Dense security delta: `(my_security - opp_security) * 0.01`.
- Dense board DP delta: `(my_total_DP - opp_total_DP) * 0.0001`.
- `GauntletWrapper` can add a bounty bonus on terminal wins against high-threat opponents.

Truncation:

- Safety cap is derived from `max_turns`.

## 3.3 Action Masking Deep Dive

Masking must be preserved end to end:

1. Engine exposes `get_action_mask(...)` with `ACTION_SPACE_SIZE` entries.
2. `DigimonEnv` thresholds the engine mask and exposes `info["action_mask"]`.
3. `ActionMasker` is the outermost training wrapper.
4. `MaskablePPO` reads masks during rollout collection and training.
5. `MaskableRecurrentPPO` stores masks in `MaskableRecurrentRolloutBuffer`.
6. Inference must pass `action_masks` into `.predict()`.

The maskable distribution sets illegal-action logits to negative infinity. Illegal actions should have zero probability during sampling and log-prob evaluation, and entropy should exclude masked actions.

## 3.4 Wrapper Chain

Full pilot training wrapper chain, innermost to outermost:

```text
DigimonEnv
  -> OpponentWrapper
  -> DeckPoolWrapper
  -> GauntletWrapper
  -> ActionMasker
```

`DeckPoolWrapper` and `GauntletWrapper` are conditional based on training configuration.

### OpponentWrapper

Location: `pilot_training.OpponentWrapper`.

Converts the two-player game into a single-agent MDP:

- Agent controls Player 1.
- Player 2 is auto-played through a configurable `opponent_fn`.
- Handles Player 2 going first after `reset()`.
- Discards dense rewards from opponent auto-steps.
- Only terminal rewards from opponent sequences pass through.

### DeckPoolWrapper

Location: `code/digimon_gym/agents/deck_pool.py`.

Varies the agent deck per episode for robustness:

- `reset()` samples a variant and injects it as `options["deck1"]`.
- `"eager"` mode samples uniformly from pre-generated variants.
- `"hybrid"` mode samples mostly from pre-generated variants and sometimes generates new variants dynamically.
- `analyze_core()` treats max-copy cards as core and remaining main-deck cards as flex; Digi-Eggs are excluded from both.
- Variant generation modifies flex slots only and preserves the 50-card main deck constraint.

### GauntletWrapper

Location: `code/digimon_gym/agents/gauntlet.py`.

Samples opponent decks from `MetaGauntlet`:

- `reset()` injects the sampled opponent deck as `options["deck2"]`.
- Adds `opponent_archetype` and `opponent_threat_index` to `info`.
- Adds `bounty_bonus` on terminal wins against opponents above `bounty_threshold`.

### LeagueOpponentWrapper

Location: `code/digimon_gym/agents/league_wrapper.py`.

Used by Stage 2 of `GauntletOrchestrator`:

- `meta_weighted`: samples by meta share with a small minimum weight.
- `pfsp`: inverse win-rate sampling, uniform until enough games have been observed.
- Tracks per-opponent games and wins on terminal episodes.

---

# 4. MetaGauntlet

Location: `code/digimon_gym/agents/gauntlet.py`.

Purpose: sample opponent decks with meta-aware weighting for training and evaluation.

## 4.1 Threat Index

If `digilab_times_played >= confidence_min_appearances`:

```text
TI = (digilab_meta_share * alpha) + (digilab_conversion_rate * beta)
```

Otherwise:

```text
TI = digilab_meta_share * alpha
```

Default parameters:

- `alpha = 1.0`.
- `beta = 2.0`.
- `confidence_min_appearances = 5`.

## 4.2 Sleeper Rule

If all conditions are true:

1. `digilab_times_played >= confidence_min_appearances`.
2. `digilab_conversion_rate > sleeper_threshold`.
3. Current sampling probability is below `sleeper_floor`.

Then the archetype probability is raised to `sleeper_floor`, and non-sleeper weights are reduced proportionally.

## 4.3 Survivorship Bias Guard

- Statistical weights come from DigiLab tournament log data.
- Scraper sources contribute decklists, not meta-share or conversion-rate weights.
- `digilab_meta_share = digilab_times_played / total_across_all_archetypes`.

## 4.4 Deck Pool Routing

Within an archetype, prefer higher-quality deck sources:

```text
digilab > digimonmeta > egman > digimoncard_io > file/manual/test
```

Keep this priority aligned with the actual loader/optimizer code when source names change.

Pipeline:

```text
code/tools/meta_loader.py -> data/deck_library.json -> MetaGauntlet.load()
```

---

# 5. GauntletOrchestrator (DB-Backed Pipeline)

Location: `code/server/workers/gauntlet_orchestrator.py`.

This is hosted-API infrastructure, not standalone training CLI code. It requires the FastAPI backend, database, and `TrainingJobWorker`.

Stage flow:

```text
configuring -> stage_1 -> stage_2 -> stage_3 -> completed
                                      \-> failed if more than 50% of stage jobs fail
```

Stage 1: Bootstrap

- All participants train against the greedy opponent.
- Creates an `Agent` plus `TrainingJob(job_type="train_vs_greedy")` per participant.
- Timesteps are derived from `stage1_games * STEPS_PER_GAME_ESTIMATE`.

Stage 2: Meta training

- Core agents use meta-weighted sampling.
- Supporting agents use PFSP-style sampling over core agents.
- Creates `TrainingJob(job_type="train_vs_agent")`.
- Stores `opponent_pool` in `config_json`.

Stage 3: Evaluation

- Runs round-robin matchups between core agents.
- Creates `TrainingJob(job_type="evaluate")` per matchup pair.
- Finalization computes matchup matrix, ETWR, and rankings.

Transitions:

- `on_job_finished()` advances stages automatically.
- Optimistic locking prevents duplicate transitions from parallel job completions.
- If more than half of a stage's jobs fail, the gauntlet is marked failed.

---

# 6. Training Job Worker

Location: `code/server/workers/training_worker.py`.

Hosted-API queue worker responsibilities:

- Poll queued `TrainingJob` rows.
- Claim jobs with DB coordination.
- Bound concurrent execution through `TRAINING_WORKER_MAX_CONCURRENT`.
- Round-robin devices across CUDA GPUs or CPU fallback.
- Recover stale jobs from other workers after the configured timeout.
- Notify `GauntletOrchestrator` when gauntlet-linked jobs finish.
- Atomically increment agent wins/losses/draws/timesteps.

Implementation note: the queue, claiming, recovery, stats, and gauntlet hooks are the durable pieces. Verify current implementation before assuming train/evaluate job bodies are complete.

---

# 7. Service Boundaries

Standalone agent modules must not import FastAPI, DB, auth, or hosted API code:

- `pilot_training.py`
- `gauntlet.py`
- `deck_pool.py`
- `features_extractor.py`
- `maskable_recurrent/`
- `architect_*.py`

DB-dependent agent modules belong to hosted API infrastructure:

- `training_worker.py`
- `gauntlet_orchestrator.py`

Engine-only routers and desktop/Tauri paths must not depend on `digimon_gym` training internals. Desktop gameplay, inference, and deck tools run through Tauri `invoke()` into `digimon-engine`, and the Tauri build must not link a Python runtime.

Legacy Python engine imports are transitional. Do not add new production imports from `engine_py_legacy.*` unless a parity/fallback path is already explicitly documented.

---

# 8. Model Artifacts and Inference

Training outputs:

- Pilot training defaults to `<repo_root>/models/<run_id>/`.
- The hosted API's `/models/manifest.json` scans `models/`.
- Architect training defaults to `architect_runs/`.

Policy formats:

- SB3 `.zip`: `MaskablePPO` or custom `MaskableRecurrentPPO`.
- ONNX `.onnx`: loaded through `digimon_gym.inference.onnx_policy`.
- Heuristic names: `"greedy"` and `"random"`.

ONNX export:

```bash
python code/tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx
python code/tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx
```

Recurrent inference rule:

- SB3 LSTM and ONNX LSTM policies must reset recurrent state at episode boundaries.
- Evaluation loops must thread both state and episode-start flags correctly.

---

# 9. Commands

Run from repo root unless noted.

```bash
# Install
pip install -r requirements.txt
pip install -e .

# Pilot training
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000

# Architect training
python -m digimon_gym.agents.architect_training --archetype "Medusamon" --episodes 200

# Env smoke check
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"

# Rust backend PyO3 bindings
cd code/digimon-engine-py && maturin develop

# Rust-backend parity test
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v

# RL tests
python -m pytest code/tests/rl -v
```

Do not run Uvicorn with `--reload` for long-running training workers; reload creates child processes that are not appropriate for worker jobs.

---

# 10. Implementation Rules for Contributors

1. Keep tensor/action contracts synchronized with `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md`, Rust constants, env wrappers, and frontend constants.
2. Preserve headless-first game logic; UI reflects state and never owns rules.
3. Maintain legal action masking for every decision step.
4. Every engine choice that affects gameplay must flow through actions or pending selection.
5. Do not bypass masks in training, simulation, inference, greedy policy code, or test helpers.
6. Recurrent loops must thread `(state, episode_start)` correctly.
7. Reset recurrent state to `None` at episode boundaries.
8. `OpponentWrapper` discards dense rewards from opponent steps; only terminal rewards pass through.
9. WebSocket state broadcasts must use `state_filter.py`; never send raw `to_ui_json()` to network clients.
10. `state_filter.py` must redact both `handIds` and `handCards` for opponents.
11. Engine-only routers must not import DB/auth/AI pipeline modules.
12. Training CLI modules must not import DB/auth/FastAPI modules.
13. Desktop builds use `VITE_BUILD_TARGET=desktop` to tree-shake admin/training UI.
14. New Rust card effects are TDD: write a failing behavioral test under `code/digimon-engine/tests/` before implementing the `CardEffect`.
15. Do not author new Python card scripts for cards already implemented in Rust.
16. All source code lives under `code/`; do not add new top-level source directories.

---

# 11. Generated Script Promotion Workflow

This workflow is for legacy Python card scripts that still need promotion into the frozen Python lane during the migration. Cards migrate one direction only: Python legacy reference to Rust ownership.

## Source/Target Layout

- Generated source: `code/engine_py_legacy/engine/data/scripts/generated/<set_id>/<module_name>.py`
- Frozen target: `code/engine_py_legacy/engine/data/scripts/<set_id>/<module_name>.py`
- Manifest: `code/engine_py_legacy/engine/data/scripts/_frozen_manifest.json`

## Promotion Contract

- Promotions must go through `promote_script_from_generated` in `code/engine_py_legacy/engine/data/script_promotion.py`.
- Promotion is hash-guarded with `expected_generated_hash`.
- Successful promotion copies the generated file to the frozen lane, updates the manifest card record, and increments the manifest version.

Single-card helper:

```bash
python code/tools/promote_script.py --card-id BT24-001 --set-id bt24 --module-name bt24_001 --expected-generated-hash <sha256>
```

Bulk promotion definition of pending:

- Generated script has no frozen counterpart.
- Frozen counterpart exists but its hash differs from generated.

Bulk run steps:

1. Scan `scripts/generated/**/*.py`, excluding `__init__.py`.
2. For each pending entry, compute the generated sha256 and call `promote_script_from_generated(...)`.
3. Re-scan and confirm pending count is `0`.

Post-promotion checks:

- `git status --short` should show frozen lane updates/additions plus manifest update.
- Pending re-scan must return `0`.

---

# 12. Documentation Index

Key docs:

- `CLAUDE.md`: current repository architecture and working rules.
- `docs/ARCHITECTURE.md`: detailed architecture reference.
- `docs/RULES_CONTEXT.md`: rules and keyword semantics.
- `docs/TENSOR_SPEC.md`: observation tensor layout.
- `docs/ACTION_SPEC.md`: action IDs and mask contract.
- `docs/TRAINING_RUNBOOK.md`: training pipeline operations.
- `docs/TOOLS.md`: CLI tools and operational workflows.
- `docs/RUST_ENGINE_API.md`: Rust card scripting API.
- `docs/RUST_PYTHON_PARITY.md`: transitional Rust/Python divergence tracker.
- `docs/RUST_ENGINE_GAPS.md`: reusable Rust scripting capability gaps surfaced by archetype audits.
- `qa/archetype-qa/engine-gaps.md`: known rule/card gaps that block no-approximations compliance.
- `.codex/skills/assess-rust-engine-archetype/`: Codex read-only DSL readiness assessment workflow for archetypes, decks, card groups, or card lists.
