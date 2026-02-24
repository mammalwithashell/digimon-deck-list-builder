AGENTS.md
Refer to RULES_CONTEXT.md for rule implementation details.

# Project Overview

The project currently has two agent tracks:

1. Architect (deck optimizer): planned, not implemented.
2. Pilot (battle agent): implemented and used in training/evaluation.

The app also includes a FastAPI backend, React frontend, and admin AI workflow used to review/fix scripts.

---

# 1. Architect (Deck Builder Agent)

Status: planned.

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

This architecture remains a spec target and is not wired into the current runtime.

---

# 2. Pilot (Battle Agent)

Goal: play Digimon matches inside the headless engine to generate win/loss and policy quality signals.

## Implemented Pilot Types

1. RL Pilot MLP (`MaskablePPO`)
- Feed-forward policy.
- Uses action masking.
- Training entrypoint: `digimon_gym/agents/pilot_training.py`.

2. RL Pilot LSTM (`MaskableRecurrentPPO` custom)
- Recurrent policy with action masking.
- Implementation: `digimon_gym/agents/maskable_recurrent/`.
- Supports threaded LSTM state during evaluation/inference.

3. Heuristic policy helpers
- `greedy_policy` lives in `digimon_gym/digimon_gym.py` and is used as a fast baseline/utility policy.

## Gym Environment Contract

Primary env: `DigimonEnv` (`digimon_gym/digimon_gym.py`).

- `reset(seed=None, options=None) -> (obs, info)`
- `step(action) -> (obs, reward, terminated, truncated, info)`
- `action_mask() -> np.ndarray[int8]`
- `get_action_mask()` alias retained for compatibility

Observation/action spaces:

- Observation: `Box(shape=(981,), dtype=float32)`
- Action: `Discrete(2120)`

Mask delivery:

- `info['action_mask']` on `reset` and `step`
- `env.action_mask()` for direct retrieval

Reward shaping:

- Terminal: win `+1.0`, loss `-1.0`, draw `0.0`
- Dense:
  - Security delta term
  - Board DP delta term

## Phase and Action Coverage

Current engine supports core and interrupt/selection phases including:

- Start, Draw, Breeding, Main, End
- SelectTarget, SelectMaterial, SelectTrash, SelectSource, SelectHand, SelectReveal, SelectEffectChoice, SelectSecurity
- BlockTiming, CounterTiming
- EndOfTurnAction
- AllianceTiming

Action space remains `2120`, with phase-dependent reuse of ID ranges. See `ACTION_SPEC.md`.

---

# 3. MetaGauntlet

Implemented in `digimon_gym/agents/gauntlet.py`.

Purpose: sample opponent decks with meta weighting for training and evaluation.

- Threat-index weighted archetype sampling
- Deck source preference routing
- Optional bounty rewards through wrapper integration in training

Deck library pipeline: `tools/meta_loader.py` -> `digimon_gym/engine/data/deck_library.json`.

---

# 4. Current App Integrations

The agent stack now runs inside a broader app platform:

- Backend API (`digimon_gym/api.py`):
  - gameplay (`/games`, `/simulations`, `/recordings`, `/replays`)
  - deck utilities (`/decks/parse`, `/decks/validate`)
  - DB-backed user/auth/deck/friends/issues/admin routes
- Frontend (`frontend/`):
  - game UI, deck builder, auth, admin pages
- Admin AI pipeline (`digimon_gym/ai/*`, `/admin/*` routes):
  - task queueing/execution
  - batch orchestration
  - fix apply, promotion audit, backlog management

This means Pilot outputs now feed both RL workflows and operational review/fix workflows.

---

# 5. Implementation Rules for Contributors

1. Strict typing and serializable game state for ML workflows.
2. Headless-first game logic (UI is a client/view layer).
3. Legal action masking must be maintained for every decision step.
4. Recurrent evaluation loops must thread `(state, episode_start)` correctly.
5. Keep tensor and action contracts synchronized with `TENSOR_SPEC.md` and `ACTION_SPEC.md`.

---

# 6. Generated Script Promotion Workflow

Use this workflow to promote generated scripts into frozen production lanes.

## Source/Target Layout

- Generated source: `digimon_gym/engine/data/scripts/generated/<set_id>/<module_name>.py`
- Frozen target: `digimon_gym/engine/data/scripts/<set_id>/<module_name>.py`
- Manifest: `digimon_gym/engine/data/scripts/_frozen_manifest.json`

## Promotion Contract

- Promotions must go through `promote_script_from_generated` (`digimon_gym/engine/data/script_promotion.py`).
- Promotion is hash-guarded using `expected_generated_hash` (sha256 of generated file).
- Successful promotion:
  - copies generated file to frozen lane
  - updates the card record in `_frozen_manifest.json`
  - increments manifest version

## Single-Card Promotion

Use the CLI helper:

`python tools/promote_script.py --card-id BT24-001 --set-id bt24 --module-name bt24_001 --expected-generated-hash <sha256>`

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
