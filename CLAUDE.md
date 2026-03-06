# CLAUDE.md - AI Assistant Guide

## Scope

This document is a current-state engineering guide for working in this repository.
It focuses on stable contracts and implementation shape, not static snapshot metrics.

## System Overview

The repository contains five major surfaces:

1. Headless Digimon game engine (`digimon_gym/engine/`)
2. RL environment and pilot training (`digimon_gym/digimon_gym.py`, `digimon_gym/agents/`)
3. FastAPI backend (`digimon_gym/api.py`, `digimon_gym/routers/`, `digimon_gym/db/routers/`)
4. React frontend (`frontend/src/`)
5. Tauri v2 desktop shell (`src-tauri/`, `digimon_gym/desktop_main.py`)

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
- `digimon_gym/engine/state_filter.py`: per-recipient hidden information filtering for network play
- `digimon_gym/engine/onnx_policy.py`: ONNX-based inference wrapper (no PyTorch required)
- `digimon_gym/routers/ws_manager.py`: WebSocket connection manager for PvP games
- `digimon_gym/routers/ws_games.py`: WebSocket game endpoint (player/spectator)
- `digimon_gym/routers/lobby.py`: game lobby with join codes and public game browser
- `digimon_gym/desktop_main.py`: lightweight desktop sidecar entry point (no DB/auth)
- `src-tauri/`: Tauri v2 desktop app shell (Rust sidecar lifecycle management)
- `scripts/export_onnx.py`: SB3 → ONNX model conversion (MLP + LSTM)
- `scripts/build-sidecar.sh`: desktop sidecar build pipeline (PyInstaller + Tauri naming)
- `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md`, `AGENTS.md`, `docs/TRAINING_RUNBOOK.md`: behavior contracts
- `docs/plans/DESKTOP_DISTRIBUTION_PLAN.md`: full implementation plan for desktop distribution
- `docs/TOOLS.md`: card registry, autoencoder, tensor layout, and new-set workflow documentation
- `digimon_gym/engine/data/tensor_layout.py`: card ID / scalar position map for FeaturesExtractor
- `digimon_gym/engine/data/card_features.py`: card feature vectorizer for autoencoder
- `digimon_gym/engine/data/card_registry.py`: card ID → integer index mapping
- `tools/build_registry.py`: append-only card registry builder (DigimonCard.io API)
- `tools/train_card_autoencoder.py`: warm-start embedding generator

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

- Tensor size: `1375` (compact layout with integer card IDs)
- Card identities are integer registry indices (1 float per card)
- `nn.Embedding` lookup happens inside `CardEmbeddingExtractor` on the GPU
- `FIELD_SLOTS=14`, `MAX_SOURCES=11`, `SLOT_SIZE=40`
- See `docs/TENSOR_SPEC.md` for exact layout

### Action Contract

- Action space size: `2168`
- `SECURITY_TARGET=14`, `BREEDING_SLOT=14` (= `FIELD_SLOTS`)
- `SOURCES_PER_FIELD=12` (stride for source selection)
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
  - `/games`, `/games/models`
  - `/recordings`
  - `/replays`
  - `/decks/parse`, `/decks/validate` (deck tools)
  - `/lobby/*` (create/join/list/cancel)
  - `/ws/games/{id}` (WebSocket PvP + spectating)

### Gameplay Routes

Primary routes include:

- Game session lifecycle: `/games`, `/games/{id}/actions`, `/games/{id}/steps`, `/games/{id}/state`, `/games/{id}/action-mask`, `/games/{id}/actions`, `/games/{id}/logs`, `/games/{id}`
- Recording/replay:
  - `/games/{id}/recording`, `/games/{id}/recordings`
  - `/recordings/*`
  - `/replays/*`

Legacy aliases are present in several routers for compatibility.

### WebSocket PvP & Spectating

- `/ws/games/{id}?token=JWT&role=player|spectator` — real-time game transport
- `ConnectionManager` (ws_manager.py) tracks players/spectators per game
- `state_filter.py` provides per-recipient hidden information filtering:
  - Players see own hand, opponent's hand hidden (count only), both security stacks hidden
  - Spectators in `"hidden"` mode see redacted state; `"open"` mode shows everything
- Message protocol: `state_update`, `player_joined`, `player_disconnected`, `game_over`, `error`
- Reconnection: frontend hook retries with exponential backoff (1s–30s, max 5 retries)

### Lobby System

- `POST /lobby/create` — creates pending game with 6-char join code (requires auth)
- `POST /lobby/join/{code}` — joins and starts InteractiveGame (both humans)
- `GET /lobby/games` — lists public pending games
- `DELETE /lobby/{id}` — host cancels pending game
- In-memory storage (`pending_games` dict)

### ONNX Agent Inference

- `CreateGameRequest` supports `player1_policy="trained"` with `player1_model="model.onnx"`
- ONNX models resolved from `ONNX_MODELS_DIR` env var (default: `models/`)
- `GET /games/models` — lists available `.onnx` files
- Model type auto-detected from filename: `*lstm*` → `OnnxLstmPolicy`, else `OnnxMlpPolicy`
- Path traversal protection via `Path.name` sanitization
- Export script: `scripts/export_onnx.py` converts SB3 .zip → .onnx (requires PyTorch)

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
- Auth-guarded: `/game/:id?`, `/deckbuilder/:id?`, `/lobby`
- Admin role-guarded: `/admin/issues`, `/admin/tasks`, `/admin/promotions`, `/admin/barracks`, `/admin/arena`, `/admin/gauntlet/:id?`, `/admin/deck-pools/:id?`

### Main Pages

- `GamePage`: play/session UI (dual-mode: HTTP for local games, WebSocket for PvP/spectating)
- `LobbyPage`: multiplayer lobby (create/join/browse tabs)
- `DeckBuilderPage`: deck editing and validation
- `AdminIssuesPage`, `AdminTasksPage`, `AdminPromotionsPage`: admin AI workflow UI
- `BarracksPage`, `ArenaPage`, `GauntletPage`, `DeckPoolPage`: training management UI

### Frontend API Architecture

- `client.ts` exports `default` (remote server) and `getGameClient()` for Tauri dual-server routing
- `useWebSocketGame.ts`: WebSocket hook for PvP/spectating with reconnection
- `lobbyApi.ts`: lobby REST client
- In Tauri desktop mode, local game requests route to sidecar (`localhost:8321`); online features to remote server

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

## Desktop Distribution (Tauri v2)

### Architecture

The desktop app uses a **dual-server** model:
- **Local sidecar** (PyInstaller binary): game engine + deck tools only, no DB/auth. For offline play against agents.
- **Remote server**: PvP, auth, lobby, user data. All online features.

### Key Files

- `src-tauri/tauri.conf.json`: build config, sidecar + resource bundling
- `src-tauri/src/main.rs`: Rust entry point, spawns/kills Python sidecar
- `digimon_gym/desktop_main.py`: stripped-down FastAPI app (no SQLAlchemy imports)
- `desktop.spec`: PyInstaller spec excluding heavy deps (torch, SB3, SQLAlchemy)
- `scripts/build-sidecar.sh`: build script with `gameplay` (no models) and `full` (ONNX) profiles
- `requirements-desktop.txt`: minimal deps for sidecar

### Build Profiles

- `gameplay` (default): greedy/random bots only, ~60-90MB
- `full`: auto-exports SB3 → ONNX, bundles models, ~90-120MB

### Working Rules for Desktop

1. `desktop_main.py` must never import from `digimon_gym.db.*` or `digimon_gym.ai.*`
2. The sidecar has its own inline game routes (not shared with `games.py`) to avoid DB import chains
3. ONNX model paths are resolved at game creation time via `ONNX_MODELS_DIR` env var
4. The Rust sidecar manager pipes stdout/stderr for debugging; sidecar is killed on window close

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

# ONNX model export (requires PyTorch)
python scripts/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx
python scripts/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx

# Desktop sidecar build
./scripts/build-sidecar.sh gameplay   # greedy bots only
./scripts/build-sidecar.sh full       # auto-exports ONNX + bundles models

# Desktop sidecar (standalone, for testing)
python -m digimon_gym.desktop_main --port 8321 --models-dir ./models

# Tauri desktop app (requires Rust toolchain)
cd src-tauri && cargo tauri dev    # development
cd src-tauri && cargo tauri build  # production installers
```

## Working Rules

1. Keep tensor and action specs in sync with `game.py` and frontend constants.
2. Preserve headless engine behavior; UI reflects state, it does not own rules.
3. Do not bypass action masking in agent logic.
4. When updating phases/actions, update tests and both spec docs in the same change.
5. Keep docs stable: avoid stale hardcoded snapshot claims unless explicitly time-stamped.
6. When threading LSTM state during evaluation/inference, reset state to `None` at episode boundaries.
7. OpponentWrapper discards dense rewards from opponent steps; only terminal rewards pass through.
8. `desktop_main.py` must not import any `digimon_gym.db.*` or `digimon_gym.ai.*` modules (breaks without SQLAlchemy).
9. WebSocket state broadcasts must use `state_filter.py` — never send raw `to_ui_json()` to network clients.
10. ONNX policies must call `reset()` at episode boundaries for LSTM models (same rule as SB3 LSTM state threading).
