# CLAUDE.md - AI Assistant Guide

## Scope

This document is a current-state engineering guide for working in this repository.
It focuses on stable contracts and implementation shape, not static snapshot metrics.

## System Overview

The repository contains four major surfaces:

1. Headless Digimon game engine (`digimon_gym/engine/`)
2. RL environment and pilot training (`digimon_gym/digimon_gym.py`, `digimon_gym/agents/`)
3. FastAPI backend (`digimon_gym/api.py`, `digimon_gym/routers/`, `digimon_gym/db/routers/`)
4. React frontend (`frontend/src/`)

It also includes an admin AI workflow for issue triage, AI task dispatch, autofix apply, and promotion auditing (`digimon_gym/ai/`, `/admin/*` routes).

## Key Repository Paths

- `digimon_gym/engine/game.py`: core rules engine, tensor writer, action mask, action decoder
- `digimon_gym/engine/data/enums.py`: phase and enum definitions
- `digimon_gym/digimon_gym.py`: `DigimonEnv` and compatibility wrapper
- `digimon_gym/agents/pilot_training.py`: MLP/LSTM pilot training entrypoint
- `digimon_gym/agents/maskable_recurrent/`: custom recurrent+mask PPO stack
- `digimon_gym/agents/gauntlet.py`: MetaGauntlet threat-index opponent sampling
- `digimon_gym/agents/gauntlet_orchestrator.py`: 3-stage training pipeline
- `digimon_gym/agents/league_wrapper.py`: PFSP opponent wrapper
- `digimon_gym/agents/deck_pool.py`: deck variant generation for training
- `digimon_gym/agents/training_worker.py`: async DB-backed training job queue
- `digimon_gym/api.py`: app assembly and router registration
- `digimon_gym/routers/`: gameplay-facing routers
- `digimon_gym/db/routers/`: auth/decks/friends/users/issues/admin routers
- `digimon_gym/ai/`: dispatcher, worker, retrieval, batch orchestrator, apply engine
- `frontend/src/App.tsx`: route map
- `frontend/src/pages/`: primary UI pages
- `frontend/src/api/`: backend API clients
- `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md`, `AGENTS.md`, `docs/TRAINING_RUNBOOK.md`: behavior contracts

## RL and Game Contracts

### Environment API

`DigimonEnv` (Gymnasium):

- `reset(seed=None, options=None) -> (obs, info)`
- `step(action) -> (obs, reward, terminated, truncated, info)`
- `action_mask() -> np.ndarray[int8]`
- `info['action_mask']` is returned from reset/step

### Reward Shaping

- Terminal: win `+1.0`, loss `-1.0`, draw `0.0`
- Dense (per-step): security delta `* 0.01`, board DP delta `* 0.0001`
- Bounty bonus (via GauntletWrapper): configurable on terminal wins vs high-TI opponents

### Tensor Contract

- Tensor size: `981`
- See `docs/TENSOR_SPEC.md` for exact layout

### Action Contract

- Action space size: `2120`
- Phase-aware decoding in `Game.decode_action`
- See `docs/ACTION_SPEC.md` for ranges and conventions

### Phase Coverage

Current `GamePhase` values include core, selection, and interrupt phases:

- `Start`, `Draw`, `Breeding`, `Main`, `End`
- `SelectTarget`, `SelectMaterial`, `SelectTrash`, `SelectSource`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`
- `BlockTiming`, `CounterTiming`
- `EndOfTurnAction`, `AllianceTiming`
- `Mulligan` (value 17)

### Wrapper Chain

Training wrapper chain (innermost to outermost):

```
DigimonEnv → OpponentWrapper → DeckPoolWrapper → GauntletWrapper → ActionMasker
```

- `OpponentWrapper`: converts 2-player game to single-agent MDP
- `DeckPoolWrapper`: varies agent's own deck per episode
- `GauntletWrapper`: samples opponent decks from MetaGauntlet
- `ActionMasker`: SB3 mask interface

Full details in `AGENTS.md` §2.4.

### Training Pipeline

- MetaGauntlet: threat-index weighted opponent sampling (see `AGENTS.md` §3)
- GauntletOrchestrator: 3-stage pipeline — bootstrap, meta-weighted/PFSP, round-robin evaluation
- Training operations: see `docs/TRAINING_RUNBOOK.md`

## Backend API Surface

### App Assembly

`digimon_gym/api.py` mounts:

- DB-backed routers:
  - `/auth/*`
  - `/users/*`
  - `/decks/*`
  - `/friends/*`
  - `/assets/*`
  - `/issues/*`
  - `/admin/*`
- Domain routers:
  - `/health`
  - `/simulations`
  - `/games`
  - `/recordings`
  - `/replays`
  - `/decks/parse`, `/decks/validate` (deck tools)

### Gameplay Routes

Primary routes include:

- Game session lifecycle: `/games`, `/games/{id}/actions`, `/games/{id}/steps`, `/games/{id}/state`, `/games/{id}/action-mask`, `/games/{id}/actions`, `/games/{id}/logs`, `/games/{id}`
- Recording/replay:
  - `/games/{id}/recording`, `/games/{id}/recordings`
  - `/recordings/*`
  - `/replays/*`

Legacy aliases are present in several routers for compatibility.

### Admin AI Routes

`/admin/*` currently supports:

- AI tasks (`/ai-tasks` create/list/get/retry/apply-fix)
- AI batches (`/ai-batches` create/preview/list/detail/cancel)
- Issue queueing (`/issues/{issue_id}/queue-fix`)
- Promotions (`/promotions`, task promotion)
- Engine backlog (`/engine-backlog`)

## Frontend Surface

### Routes

`frontend/src/App.tsx` defines:

- Public: `/`, `/login`, `/register`
- Auth-guarded: `/game/:id?`, `/deckbuilder/:id?`
- Admin role-guarded: `/admin/issues`, `/admin/tasks`, `/admin/promotions`, `/admin/barracks`, `/admin/arena`, `/admin/gauntlet/:id?`, `/admin/deck-pools/:id?`

### Main Pages

- `GamePage`: play/session UI
- `DeckBuilderPage`: deck editing and validation
- `AdminIssuesPage`, `AdminTasksPage`, `AdminPromotionsPage`: admin AI workflow UI
- `BarracksPage`, `ArenaPage`, `GauntletPage`, `DeckPoolPage`: training management UI

### Frontend Action/Phase Constants

- `frontend/src/utils/constants.ts`
- `frontend/src/utils/actionDecoder.ts`

Keep these aligned with backend constants.

## Admin AI Workflow (Current)

Core modules:

- `dispatcher.py`: task-specific prompt+schema dispatch
- `worker.py`: DB-backed queue loop and execution
- `batch_orchestrator.py`: batch creation/scheduling/guards/finalization
- `autofix_apply.py`: scoped edit validation + apply + checks
- `git_adapter.py`: worktree/branch/commit/PR automation

Common task types:

- `review_batch`
- `qa_analysis`
- `engine_audit`
- `script_autofix`

Common scope profiles:

- `script`
- `script_engine`
- `script_engine_transpiler`

## Commands

Run from repo root unless noted.

```bash
# Install
pip install -r requirements.txt

# Backend API (development)
python -m uvicorn digimon_gym.api:app --reload --reload-dir digimon_gym

# Backend API (production / long-running tasks)
# NOTE: Do NOT use --reload for long-running tasks (creates zombie processes)
python -m uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000

# Frontend
cd frontend
npm install
npm run dev

# Tests
python -m pytest tests -v

# Targeted tests
python -m pytest tests/test_tensor_and_actions.py -v
python -m pytest tests/test_phase_decoders.py -v
python -m pytest tests/test_maskable_recurrent.py -v

# RL training
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000

# Env smoke check
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
```

## Working Rules

1. Keep tensor and action specs in sync with `game.py` and frontend constants.
2. Preserve headless engine behavior; UI reflects state, it does not own rules.
3. Do not bypass action masking in agent logic.
4. When updating phases/actions, update tests and both spec docs in the same change.
5. Keep docs stable: avoid stale hardcoded snapshot claims unless explicitly time-stamped.
6. When threading LSTM state during evaluation/inference, reset state to `None` at episode boundaries.
7. OpponentWrapper discards dense rewards from opponent steps; only terminal rewards pass through.
