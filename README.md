# Digimon TCG Deck List Builder

Status: pre-alpha, active development.

This repository is a Digimon TCG platform with a headless rules engine, RL training environment, backend API, frontend UI, and admin AI workflow for script review/fix operations.

## What Is Implemented

- Headless game engine (`digimon_gym/engine/`)
- Gymnasium RL environment (`DigimonEnv`)
- Pilot training stack (MLP + recurrent maskable PPO)
- FastAPI backend with gameplay and DB-backed routers
- React frontend with gameplay, deckbuilder, auth, and admin pages
- Admin AI task/batch/apply/promotion workflow

## Architecture

### Engine and RL

- Engine core: `digimon_gym/engine/game.py`
- Tensor contract: `981` floats (`TENSOR_SPEC.md`)
- Action contract: `2120` actions (`ACTION_SPEC.md`)
- Gym env: `digimon_gym/digimon_gym.py`

### Backend API

App entrypoint: `digimon_gym/api.py`

Mounted route groups:

- Auth/users/decks/friends/assets/issues/admin (`digimon_gym/db/routers/`)
- Health/simulations/games/recordings/replays/deck-tools (`digimon_gym/routers/`)

### Frontend

- App routes: `frontend/src/App.tsx`
- Pages: gameplay, deckbuilder, auth, admin issues/tasks/promotions
- API clients: `frontend/src/api/`

### Admin AI Pipeline

- Dispatcher: `digimon_gym/ai/dispatcher.py`
- Worker: `digimon_gym/ai/worker.py`
- Batch orchestration: `digimon_gym/ai/batch_orchestrator.py`
- Scoped apply checks: `digimon_gym/ai/autofix_apply.py`
- Git/PR automation: `digimon_gym/ai/git_adapter.py`

## Quick Start

### 1) Install

```bash
pip install -r requirements.txt
```

### 2) Run Backend

```bash
python -m uvicorn digimon_gym.api:app --reload --reload-dir digimon_gym
```

### 3) Run Frontend

```bash
cd frontend
npm install
npm run dev
```

## Common Commands

```bash
# Full test suite
python -m pytest tests -v

# Focused validation
python -m pytest tests/test_tensor_and_actions.py -v
python -m pytest tests/test_phase_decoders.py -v
python -m pytest tests/test_maskable_recurrent.py -v

# Pilot training
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000

# Env smoke check
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
```

## API Surface (Summary)

Gameplay and tools:

- `POST /simulations`
- `POST /games`
- `POST /games/{game_id}/actions`
- `POST /games/{game_id}/steps`
- `GET /games/{game_id}/state`
- `GET /games/{game_id}/action-mask`
- `GET /games/{game_id}/actions`
- `GET /games/{game_id}/logs`
- `DELETE /games/{game_id}`
- `GET /recordings`, `GET /recordings/{id}`, `GET /recordings/{id}/state`
- `POST /replays`, `POST /replays/{id}/steps`, `POST /replays/{id}/seek`, `DELETE /replays/{id}`
- `POST /decks/parse`, `POST /decks/validate`

DB-backed app routes:

- `/auth/*`, `/users/*`, `/decks/*`, `/friends/*`, `/assets/*`, `/issues/*`, `/admin/*`

## Documentation Index

- `ACTION_SPEC.md`: action-space and decoder behavior
- `TENSOR_SPEC.md`: observation tensor format
- `AGENTS.md`: architect/pilot model and integration view
- `GEMINI.md`, `CLAUDE.md`: assistant engineering guides
- `docs/admin_ai_batch_runbook.md`: admin batch operations
- `RULES_CONTEXT.md`: rules context used for implementation

## Notes

- Avoid relying on static count claims in docs; implementation shape is the source of truth.
- When updating phases/actions/tensor layout, update code, specs, and tests in the same change.
