# AGENTS.md

Refer to `docs/RULES_CONTEXT.md` for rule implementation details.
Refer to `docs/TRAINING_RUNBOOK.md` for training pipeline operations.

## Project Overview

The project currently has two agent tracks:

1. Architect (deck optimizer): planned, not implemented.
2. Pilot (battle agent): implemented and used in training/evaluation.

The app also includes a FastAPI backend, React frontend, and admin AI workflow used to review/fix scripts.

---

# 1. Architect (Deck Builder Agent)

**Status: planned / roadmap.**

Intended algorithm: DQN/Q-DeckRec style deck optimization against meta opponents.

## Planned MDP

- State:
  - Candidate deck vector
  - Opponent/meta deck vector
  - Step counter
- Action:
  - Card swap `(remove_i, add_j)` with fixed deck size
- Reward:
  - Cumulative win-rate based objective (batch simulations against pilot opponents)

Roadmap items (not implemented):

- Q-DeckRec: DQN-based deck recommendation trained on simulated win rates
- CPR (Contextual Preference Ranking): dense card embeddings from autoencoder on stats/keywords
- Card embedding space: allows recommending unseen cards by semantic distance

This architecture remains a spec target and is not wired into the current runtime.

---

# 2. Pilot (Battle Agent)

Goal: play Digimon matches inside the headless engine to generate win/loss and policy quality signals.

## 2.1 Implemented Pilot Types

### MaskablePPO (MLP baseline)

- Feed-forward policy from `sb3_contrib.MaskablePPO` with default `"MlpPolicy"`.
- Uses action masking via `ActionMasker` wrapper.
- Training entrypoint: `code/digimon_gym/agents/pilot_training.py`.
- Observation: 981 floats → MLP → 2120 action logits.

### MaskableRecurrentPPO (Custom LSTM)

Custom implementation in `code/digimon_gym/agents/maskable_recurrent/` (3 modules + `__init__`).

**Motivation**: SB3's `RecurrentPPO` and `MaskablePPO` are separate algorithms with no official combination. This module merges both capabilities.

**Architecture**:

- `MaskableMlpLstmPolicy` (`policies.py`):
  - Extends `RecurrentActorCriticPolicy`.
  - Swaps `CategoricalDistribution` for `MaskableCategoricalDistribution`.
  - Separate actor/critic LSTMs (`enable_critic_lstm=True` by default).
  - `forward()` accepts `action_masks` kwarg → `distribution.apply_masking()`.
  - `evaluate_actions()` also accepts `action_masks` for training loss computation.
  - `predict()` handles numpy in/out with LSTM state management.

- `MaskableRecurrentRolloutBuffer` (`buffers.py`):
  - Extends `RecurrentRolloutBuffer` with `action_masks` storage.
  - Sequence-aware batching preserves LSTM state boundaries.

- `MaskableRecurrentPPO` (`maskable_recurrent_ppo.py`):
  - Extends `RecurrentPPO`.
  - `collect_rollouts()` modified to gather action masks each step.
  - `train()` modified to pass action masks during `evaluate_actions()`.

**Default hyperparameters** (from `pilot_training.py`):

- `lstm_hidden_size`: 256
- `n_lstm_layers`: 1
- `enable_critic_lstm`: True
- `net_arch`: `dict(pi=[64], vf=[64])`
- `batch_size`: forced = `n_steps` (RecurrentPPO requirement)

**LSTM state threading during inference**:

- `state = (h, c)` tuple passed between `predict()` calls within an episode.
- Reset to `None` at episode boundaries.
- Closure pattern in `make_agent_opponent_fn()` for opponent LSTM state.

### Greedy Policy (Heuristic)

Location: `digimon_gym.digimon_gym.greedy_policy()`.

Used as default opponent during training and fast baseline. Priority logic by phase:

- **Mulligan**: checks hand for level-3 digimon; mulligan (action 1) if none, keep (action 0) if found.
- **Breeding**: hatch > move > pass.
- **Main**: keep-turn digivolve > attack > play > pass.
  - Attack scoring: lethal priority 3, favorable DP priority 2, unfavorable 0.
  - Digivolve scoring: only digivolves that keep turn (cost ≤ relative memory).
- **Selection (TRASH_CARD)**: dump lowest-value card first.
- **Default/other phases**: pass or first valid action.

### Random Policy

`pilot_training.random_policy()`: uniform sample from valid actions. Fallback: `ACTION_PASS_TURN` if no valid actions.

## 2.2 Action Masking Deep Dive

How masking works end-to-end:

1. **Engine**: `Game.get_action_mask(player_id)` → `float32[2120]` (1.0 = legal, 0.0 = illegal). Phase-aware: different actions are legal in different phases.
2. **DigimonEnv**: `action_mask()` thresholds at 0.5, returns `bool[2120]`.
3. **Info dict**: `info['action_mask']` included in both `reset()` and `step()`.
4. **Wrapper chain**: `ActionMasker(env, mask_fn)` at the outermost layer. `mask_fn` unwraps to `DigimonEnv` and calls `action_mask()`.
5. **MaskablePPO**: reads masks during `collect_rollouts()`, applies during `train()`.
6. **MaskableRecurrentPPO**: same, but also stores masks in `MaskableRecurrentRolloutBuffer`.
7. **Inference**: `predict()` accepts `action_masks` kwarg, applies to distribution.

**Masking in the distribution layer**: `MaskableCategoricalDistribution` sets logits of masked actions to `-inf`. This guarantees zero probability for illegal actions during both sampling (exploration) and `log_prob` computation (policy gradient). Entropy computation excludes masked actions.

## 2.3 Gym Environment Contract

Primary env: `DigimonEnv` (`code/digimon_gym/digimon_gym.py`).

### API

- `reset(seed, options)` → `(obs, info)`. Options supports: `deck1`, `deck2` (per-episode override).
- `step(action)` → `(obs, reward, terminated, truncated, info)`. Truncation safety: `max_turns * 10` steps.
- `action_mask()` → `bool[2120]`
- `get_action_mask()` alias retained for compatibility.

### Observation/Action Spaces

- Observation: `Box(shape=(981,), low=-10.0, high=10000.0, dtype=float32)`. See `docs/TENSOR_SPEC.md`.
- Action: `Discrete(2120)`. See `docs/ACTION_SPEC.md`.

### Mask Delivery

- `info['action_mask']` on `reset` and `step`.
- `env.action_mask()` for direct retrieval.

### Reward Shaping

- **Terminal**: win `+1.0`, loss `-1.0`, draw `0.0`.
- **Dense (per-step)**:
  - Security delta: `(my_security - opp_security) * 0.01`
  - Board DP delta: `(my_total_DP - opp_total_DP) * 0.0001`
- **Bounty bonus** (via GauntletWrapper): configurable bonus on terminal wins vs high-TI opponents.

### PendingAction Enum

- `NO_ACTION = 0`: no pending action.
- `TRASH_CARD = 1`: must select card(s) to trash.
- *(Roadmap: additional pending action types planned.)*

## 2.4 Wrapper Chain

Full training wrapper chain (innermost to outermost):

```
DigimonEnv
  → OpponentWrapper        (converts 2-player to single-agent MDP)
  → DeckPoolWrapper         (varies agent's own deck per episode)
  → GauntletWrapper         (samples opponent decks from MetaGauntlet)
  → ActionMasker            (SB3 mask interface)
```

### OpponentWrapper

Location: `pilot_training.OpponentWrapper`.

Converts 2-player game to single-agent MDP. Auto-plays Player 2 turns using configurable `opponent_fn`. Reward attribution: only terminal rewards from opponent sequences pass through; dense shaping from opponent moves is discarded. Handles Player 2 going first after `reset()`.

### DeckPoolWrapper

Location: `code/digimon_gym/agents/deck_pool.DeckPoolWrapper`.

Varies agent's deck per episode for robustness training. On `reset()`, samples a variant from the pool and injects via `options["deck1"]`.

**Modes**:
- `"eager"`: uniform sample from pre-generated variants.
- `"hybrid"`: 80% pre-generated, 20% on-the-fly generation (capped at `hybrid_max_dynamic`, default 10).

**Core/flex analysis**: `analyze_core()` identifies max-copy cards as core, rest as flex. Digi-Egg cards excluded from both (separate deck zone).

**Variant generation**: `generate_variants()` applies stochastic count-adjust and side-swap on flex slots only. Number of modifications scales with flex pool size (1-3, 2-5, or 3-8). Validates 50-card constraint and deduplicates.

### GauntletWrapper

Location: `code/digimon_gym/agents/gauntlet.GauntletWrapper`.

Samples opponent deck from MetaGauntlet on `reset()`. Injects sampled deck via `options["deck2"]`.

- **Bounty reward**: `+bounty_bonus` (default 0.5) on terminal win vs opponent with `TI > bounty_threshold` (default 0.15).
- **Info enrichment**: `opponent_archetype`, `opponent_threat_index`.

### LeagueOpponentWrapper

Location: `code/digimon_gym/agents/league_wrapper.LeagueOpponentWrapper`.

Used in Stage 2 of GauntletOrchestrator pipeline. Two sampling modes:

- **meta_weighted**: weight = `meta_share` from DigiLab stats. Minimum weight 0.01.
- **pfsp**: inverse win-rate weighting (targets weakest matchups). Uniform until 5+ games per opponent, then `weight = max(0.01, 1.0 - win_rate)`.

Tracks per-opponent game/win counts during `step()` on terminal episodes.

---

# 3. MetaGauntlet

Location: `code/digimon_gym/agents/gauntlet.py`.

Purpose: sample opponent decks with meta weighting for training and evaluation.

## 3.1 Threat Index Formula

```
if digilab_times_played >= confidence_min_appearances (default: 5):
    TI = (digilab_meta_share * alpha) + (digilab_conversion_rate * beta)
else:
    TI = digilab_meta_share * alpha   # insufficient data for conversion
```

Default parameters: `alpha=1.0`, `beta=2.0`.

## 3.2 Sleeper Rule

If ALL three conditions met:

1. `digilab_times_played >= confidence_min_appearances` (5)
2. `digilab_conversion_rate > sleeper_threshold` (0.50)
3. Current sampling probability < `sleeper_floor` (0.05)

Then: force sampling probability to `sleeper_floor` (5%). Redistribution: proportionally reduce non-sleeper weights.

## 3.3 Survivorship Bias Fix

- Statistical weights (TI) derived ONLY from DigiLab tournament log data (full field participation counts).
- Scraper sources (DigimonMeta, Egman Events) contribute decklists only, NOT the `meta_share` / `conversion_rate` used to weight sampling.
- `digilab_meta_share` computed as: `digilab_times_played / total_across_all_archetypes`.

## 3.4 Deck Pool Routing

When sampling within an archetype, prefer higher-quality deck sources:

```
digimonmeta (3) > egman (2) > digimoncard_io (1) > file/manual/test (0)
```

Decks sorted by source preference; position-biased within archetype.

## 3.5 Deck Library Pipeline

`code/tools/meta_loader.py` → `data/deck_library.json` → `MetaGauntlet.load()`

---

# 4. GauntletOrchestrator (3-Stage Pipeline)

Location: `code/server/workers/gauntlet_orchestrator.py`.

DB-backed training pipeline managed by `GauntletOrchestrator`. Requires running backend (FastAPI + TrainingJobWorker). Detailed operations in `docs/TRAINING_RUNBOOK.md`.

## Stage Flow

```
configuring → stage_1 (bootstrap) → stage_2 (meta training) → stage_3 (evaluation) → completed
                                                                                    → failed (if >50% jobs fail)
```

## Stage 1: Bootstrap

- All participants train vs greedy opponent.
- Creates `Agent` + `TrainingJob(job_type="train_vs_greedy")` per participant.
- Timesteps: `stage1_games * STEPS_PER_GAME_ESTIMATE` (50).

## Stage 2: Meta-Weighted Training

- **Core agents**: meta_weighted sampling (weight = opponent `meta_share`).
- **Supporting agents**: PFSP sampling (uniform over core agents).
- Creates `TrainingJob(job_type="train_vs_agent")` per participant.
- Each job has `opponent_pool` in `config_json`.

## Stage 3: Round-Robin Evaluation

- All `C(n,2)` matchups between core agents (e.g., C(8,2) = 28 for 8 agents).
- Creates `TrainingJob(job_type="evaluate")` per matchup pair.
- Finalization computes:
  - **Matchup matrix**: pairwise win rates.
  - **ETWR** (Expected Tournament Win Rate): `ETWR(A) = sum(wr(A,X) * meta_share(X)) / sum(meta_share(X))` for all X ≠ A.
  - Rankings sorted by ETWR descending.

## Stage Transitions

- Automatic on job completion via `on_job_finished()` callback.
- Failure guard: if >50% of stage jobs fail, gauntlet status → `"failed"`.
- Optimistic locking: prevent double stage transitions from parallel jobs.

---

# 5. Training Job Worker

Location: `code/server/workers/training_worker.py`.

- Async DB-backed queue worker.
- Polls for queued `TrainingJob` rows.
- Concurrent execution bounded by `TRAINING_WORKER_MAX_CONCURRENT` (default 1).
- Device management: round-robin across CUDA GPUs or CPU fallback.
- Stale job recovery: marks long-running jobs (>2h) from other workers as failed.
- Gauntlet hook: notifies `GauntletOrchestrator` when gauntlet-linked jobs finish.
- Agent stats: atomic SQL increments for wins/losses/draws/timesteps.

**Note**: `_run_heuristic_training`, `_run_agent_training`, `_run_evaluation` are currently placeholder stubs. The DB queue mechanics, job claiming, stale recovery, and gauntlet hooks are fully implemented.

---

# 6. Current App Integrations

The agent stack now runs inside a broader app platform:

- Backend API (`code/server/api.py`):
  - gameplay (`/games`, `/simulations`, `/recordings`, `/replays`)
  - deck utilities (`/decks/parse`, `/decks/validate`)
  - DB-backed user/auth/deck/friends/issues/admin routes
- Frontend (`code/frontend/`):
  - game UI, deck builder, auth, admin pages
- Admin AI pipeline (`code/server/ai/*`, `/admin/*` routes):
  - task queueing/execution
  - batch orchestration
  - fix apply, promotion audit, backlog management

This means Pilot outputs now feed both RL workflows and operational review/fix workflows.

---

# 7. Implementation Rules for Contributors

1. Strict typing and serializable game state for ML workflows.
2. Headless-first game logic (UI is a client/view layer).
3. Legal action masking must be maintained for every decision step.
4. Recurrent evaluation loops must thread `(state, episode_start)` correctly.
5. Keep tensor and action contracts synchronized with `docs/TENSOR_SPEC.md` and `docs/ACTION_SPEC.md`.
6. When threading LSTM state during evaluation/inference, reset state to `None` at episode boundaries.
7. OpponentWrapper discards dense rewards from opponent steps; only terminal rewards pass through.

---

# 8. Generated Script Promotion Workflow

Use this workflow to promote generated scripts into frozen production lanes.

## Source/Target Layout

- Generated source: `code/engine_py_legacy/engine/data/scripts/generated/<set_id>/<module_name>.py`
- Frozen target: `code/engine_py_legacy/engine/data/scripts/<set_id>/<module_name>.py`
- Manifest: `code/engine_py_legacy/engine/data/scripts/_frozen_manifest.json`

## Promotion Contract

- Promotions must go through `promote_script_from_generated` (`code/engine_py_legacy/engine/data/script_promotion.py`).
- Promotion is hash-guarded using `expected_generated_hash` (sha256 of generated file).
- Successful promotion:
  - copies generated file to frozen lane
  - updates the card record in `_frozen_manifest.json`
  - increments manifest version

## Single-Card Promotion

Use the CLI helper:

`python code/tools/promote_script.py --card-id BT24-001 --set-id bt24 --module-name bt24_001 --expected-generated-hash <sha256>`

## Bulk Promotion (Current Generated Scripts)

Definition of pending:

- any generated script whose frozen counterpart is missing, or
- frozen counterpart exists but hash differs from generated.

Bulk run steps:

1. Scan `scripts/generated/**.py` (excluding `__init__.py`) for pending entries.
2. For each pending entry, compute generated sha256 and call `promote_script_from_generated(...)`.
3. Re-scan and confirm pending count is `0`.

## Post-Promotion Checks

- `git status --short` should show frozen lane script updates/additions plus manifest update.
- Pending re-scan must return `0`.
