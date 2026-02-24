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
- `digimon_gym/api.py`: app assembly and router registration
- `digimon_gym/routers/`: gameplay-facing routers
- `digimon_gym/db/routers/`: auth/decks/friends/users/issues/admin routers
- `digimon_gym/ai/`: dispatcher, worker, retrieval, batch orchestrator, apply engine
- `frontend/src/App.tsx`: route map
- `frontend/src/pages/`: primary UI pages
- `frontend/src/api/`: backend API clients
- `TENSOR_SPEC.md`, `ACTION_SPEC.md`, `AGENTS.md`: behavior contracts

## RL and Game Contracts

### Environment API

`DigimonEnv` (Gymnasium):

- `reset(seed=None, options=None) -> (obs, info)`
- `step(action) -> (obs, reward, terminated, truncated, info)`
- `action_mask() -> np.ndarray[int8]`
- `info['action_mask']` is returned from reset/step

### Tensor Contract

- Tensor size: `981`
- See `TENSOR_SPEC.md` for exact layout

### Action Contract

- Action space size: `2120`
- Phase-aware decoding in `Game.decode_action`
- See `ACTION_SPEC.md` for ranges and conventions

### Phase Coverage

Current `GamePhase` values include core, selection, and interrupt phases:

- `Start`, `Draw`, `Breeding`, `Main`, `End`
- `SelectTarget`, `SelectMaterial`, `SelectTrash`, `SelectSource`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`
- `BlockTiming`, `CounterTiming`
- `EndOfTurnAction`, `AllianceTiming`

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
- Admin role-guarded: `/admin/issues`, `/admin/tasks`, `/admin/promotions`

### Main Pages

- `GamePage`: play/session UI
- `DeckBuilderPage`: deck editing and validation
- `AdminIssuesPage`, `AdminTasksPage`, `AdminPromotionsPage`: admin AI workflow UI

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

# Backend API
python -m uvicorn digimon_gym.api:app --reload

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

# Env smoke check
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
```

## Working Rules

1. Keep tensor and action specs in sync with `game.py` and frontend constants.
2. Preserve headless engine behavior; UI reflects state, it does not own rules.
3. Do not bypass action masking in agent logic.
4. When updating phases/actions, update tests and both spec docs in the same change.
5. Keep docs stable: avoid stale hardcoded snapshot claims unless explicitly time-stamped.
