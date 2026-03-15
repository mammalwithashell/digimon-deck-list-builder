# Digimon TCG Simulator

A headless Digimon TCG rules engine (based on DCGO) with a Gymnasium RL environment, web-playable frontend, and admin AI pipeline — all in pure Python, no Unity.

**Status**: pre-alpha, active development.

## What This Project Does

- **Headless rules engine** — 3000+ line game engine implementing DCGO rules, 60+ effect timings, keyword system, stack-based effect resolution
- **Gymnasium RL environment** — 981-float observation tensor, 2120 discrete action space with phase-aware masking, dense reward shaping
- **Web-playable UI** — React frontend with drag-and-drop gameplay, deck builder, admin dashboards
- **Multiplayer PvP** — WebSocket-based real-time gameplay with lobby matchmaking, join codes, and spectating with hidden information filtering
- **Desktop app** — Tauri v2 shell with bundled Python sidecar for offline play against AI agents (ONNX inference, no PyTorch required)
- **Admin AI pipeline** — LLM-powered card script transpilation, automated fixes, batch orchestration with safe-apply checks
- **Training infrastructure** — MetaGauntlet opponent sampling, 3-stage training pipeline, PFSP opponent weighting, deck variant generation

## Architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │              Tauri v2 Desktop Shell (Rust)                │
   │   Manages Python sidecar lifecycle, bundles frontend      │
   └──────────────────────┬───────────────────────────────────┘
                          │ spawns
   ┌──────────────────────▼───────────────────────────────────┐
   │         React Frontend (TypeScript)                       │
   │   GamePage (HTTP/WS dual-mode), LobbyPage, DeckBuilder   │
   └─────────┬──────────────────────────┬─────────────────────┘
             │ HTTP (local games)        │ WebSocket (PvP)
   ┌─────────▼─────────┐     ┌──────────▼──────────────────┐
   │  Desktop Sidecar   │     │   FastAPI Server (Python)    │
   │  (PyInstaller)     │     │   api.py + routers/          │
   │  Game engine only   │     │   + lobby + WebSocket PvP   │
   │  No DB/auth         │     │   + auth + DB               │
   └─────────┬──────────┘     └──────────┬──────────────────┘
             │                           │
             └─────────┬─────────────────┘
                       │
        ┌──────────────┼──────────────────────┐
        │              │                      │
   ┌────▼──────┐ ┌─────▼─────────┐ ┌─────────▼───────┐
   │ Game       │ │ DigimonEnv    │ │ Admin AI        │
   │ Engine     │ │ (Gymnasium)   │ │ ai/             │
   │ + ONNX     │ │ 981-obs       │ │ LLM dispatch    │
   │ inference  │ │ 2120-act      │ │ batch orch.     │
   └────┬───────┘ └─────┬─────────┘ └─────────────────┘
        │               │
        │      ┌────────▼────────┐
        │      │  RL Agents      │
        │      │  MaskablePPO    │
        │      │  LSTM + Mask    │
        │      │  MetaGauntlet   │
        │      └─────────────────┘
   ┌────▼───────────┐
   │  Card Scripts   │
   │  ~24 sets       │
   │  Python (C#)    │
   └─────────────────┘
```

## Repository Map

| Path | Purpose |
|---|---|
| `digimon_gym/engine/game.py` | Core rules engine, tensor writer, action mask, action decoder |
| `digimon_gym/engine/data/enums.py` | GamePhase, PendingAction, CardColor, etc. |
| `digimon_gym/engine/data/scripts/` | Transpiled Python card scripts (~24 sets) |
| `digimon_gym/digimon_gym.py` | DigimonEnv (Gymnasium interface) |
| `digimon_gym/agents/` | RL agents, training, wrappers |
| `digimon_gym/agents/maskable_recurrent/` | Custom LSTM + action masking PPO |
| `digimon_gym/agents/gauntlet.py` | MetaGauntlet opponent sampling |
| `digimon_gym/api.py` | FastAPI app assembly |
| `digimon_gym/routers/` | Gameplay API routes |
| `digimon_gym/routers/lobby.py` | Multiplayer lobby (join codes, public browser) |
| `digimon_gym/routers/ws_games.py` | WebSocket PvP + spectating endpoint |
| `digimon_gym/routers/ws_manager.py` | WebSocket connection manager |
| `digimon_gym/engine/state_filter.py` | Hidden information filtering (player/spectator perspectives) |
| `digimon_gym/engine/onnx_policy.py` | ONNX inference wrapper (no PyTorch) |
| `digimon_gym/desktop_main.py` | Desktop sidecar entry point (game engine only) |
| `digimon_gym/db/routers/` | Auth, decks, admin routes |
| `digimon_gym/ai/` | Admin AI task/batch pipeline |
| `src-tauri/` | Tauri v2 desktop shell (Rust sidecar management) |
| `tools/export_onnx.py` | SB3 → ONNX model conversion |
| `tools/build-sidecar.sh` | Desktop sidecar build pipeline |
| `frontend/src/` | React UI |
| `tools/` | Meta loader, transpiler, promotion CLI |

## Quick Start

### Prerequisites

- Python 3.11+
- Node.js 18+ (for frontend)

### Install

```bash
pip install -r requirements.txt
```

### Run Backend

```bash
# Development (auto-reload)
python -m uvicorn digimon_gym.api:app --reload --reload-dir digimon_gym

# Production / long-running tasks (no --reload to avoid zombie processes)
python -m uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000
```

### Run Frontend

```bash
cd frontend
npm install
npm run dev
```

### Smoke-Check the RL Environment

```bash
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
```

### Run Tests

```bash
python -m pytest tests -v
```

### Train an Agent

```bash
# MLP baseline
python -m digimon_gym.agents.pilot_training --timesteps 500000

# LSTM with memory
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000

# Self-play
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000

# MetaGauntlet opponent sampling
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000
```

### Export Trained Models to ONNX

```bash
# Export SB3 checkpoint to ONNX (requires PyTorch)
python tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx
python tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx
```

### Build Desktop App

```bash
# Prerequisites: Rust toolchain, PyInstaller
pip install pyinstaller

# Build sidecar binary (greedy bots only)
./tools/build-sidecar.sh gameplay

# Build sidecar binary (auto-exports ONNX + bundles models)
./tools/build-sidecar.sh full

# Build Tauri desktop installer
cd src-tauri && cargo tauri build
```

### Run Desktop Sidecar Standalone (for testing)

```bash
python -m digimon_gym.desktop_main --port 8321 --models-dir ./models
```

## Documentation Index

| Document | Purpose |
|---|---|
| `AGENTS.md` | RL agent architecture, wrapper chain, MetaGauntlet, GauntletOrchestrator |
| `docs/TRAINING_RUNBOOK.md` | Training pipeline operations guide |
| `docs/TENSOR_SPEC.md` | 981-float observation tensor layout |
| `docs/ACTION_SPEC.md` | 2120 action space and phase-aware decoding |
| `docs/RULES_CONTEXT.md` | Comprehensive Digimon TCG rules reference |
| `CLAUDE.md` | AI assistant engineering guide |
| `GEMINI.md` | AI assistant context (legacy) |
| `docs/UI_PLAN.md` | Frontend and API surface design |
| `docs/admin_ai_batch_runbook.md` | Admin AI pipeline operations |
| `docs/EDH_COMMANDER_MODE.md` | 4-player commander format spec |
| `docs/TITAN_MODE.md` | Asymmetric multiplayer format spec |
| `docs/plans/DESKTOP_DISTRIBUTION_PLAN.md` | Desktop distribution implementation plan (WebSocket PvP, ONNX, Tauri) |

## Roadmap

**Recently added:**

- WebSocket PvP with lobby matchmaking and spectating (hidden information filtering)
- ONNX inference for trained agents (play against ML models without PyTorch)
- Tauri v2 desktop shell with Python sidecar bundling

**Planned but not yet implemented:**

- **Q-DeckRec**: Architect agent for DQN-based deck optimization
- **CPR (Contextual Preference Ranking)**: dense card embeddings from autoencoder on stats/keywords for unseen card recommendation
- **Persistent meta-league**: cross-run league with ELO tracking
- **Extended PendingAction phases**: beyond `TRASH_CARD` for more granular selection states
- **4-player game modes**: Commander/EDH and Titan Mode (engine is currently 2-player hardcoded)

## Notes

- Avoid relying on static count claims in docs; implementation shape is the source of truth.
- When updating phases/actions/tensor layout, update code, specs, and tests in the same change.
