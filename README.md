# Digimon TCG Simulator

A headless Digimon TCG rules engine (based on DCGO) with a Gymnasium RL environment, web-playable frontend, and admin AI pipeline — all in pure Python, no Unity.

**Status**: pre-alpha, active development.

## What This Project Does

- **Headless rules engine** — 3000+ line game engine implementing DCGO rules, 60+ effect timings, keyword system, stack-based effect resolution
- **Gymnasium RL environment** — 981-float observation tensor, 2120 discrete action space with phase-aware masking, dense reward shaping
- **Web-playable UI** — React frontend with drag-and-drop gameplay, deck builder, admin dashboards
- **Admin AI pipeline** — LLM-powered card script transpilation, automated fixes, batch orchestration with safe-apply checks
- **Training infrastructure** — MetaGauntlet opponent sampling, 3-stage training pipeline, PFSP opponent weighting, deck variant generation

## Architecture

```
                         ┌─────────────────────────────┐
                         │  React Frontend (TypeScript) │
                         └──────────┬──────────────────┘
                                    │ HTTP
                         ┌──────────▼──────────────────┐
                         │   FastAPI Backend (Python)    │
                         │   api.py + routers/           │
                         └──────────┬──────────────────┘
                                    │
              ┌─────────────────────┼─────────────────────┐
              │                     │                     │
   ┌──────────▼────────┐ ┌─────────▼─────────┐ ┌────────▼────────┐
   │  Game Engine       │ │  DigimonEnv       │ │  Admin AI       │
   │  engine/game.py    │ │  (Gymnasium)      │ │  ai/            │
   │  3000+ lines       │ │  981-obs, 2120-act│ │  LLM dispatch   │
   └──────────┬─────────┘ └─────────┬─────────┘ │  batch orch.    │
              │                     │            │  safe-apply     │
              │            ┌────────▼────────┐   └─────────────────┘
              │            │  RL Agents      │
              │            │  MaskablePPO    │
              │            │  LSTM + Mask    │
              │            │  MetaGauntlet   │
              │            └─────────────────┘
              │
   ┌──────────▼─────────┐
   │  Card Scripts       │
   │  ~24 sets           │
   │  Python (from C#)   │
   └─────────────────────┘
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
| `digimon_gym/db/routers/` | Auth, decks, admin routes |
| `digimon_gym/ai/` | Admin AI task/batch pipeline |
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

## Roadmap

These features are planned but **not yet implemented**:

- **Q-DeckRec**: Architect agent for DQN-based deck optimization
- **CPR (Contextual Preference Ranking)**: dense card embeddings from autoencoder on stats/keywords for unseen card recommendation
- **Persistent meta-league**: cross-run league with ELO tracking
- **Extended PendingAction phases**: beyond `TRASH_CARD` for more granular selection states

## Notes

- Avoid relying on static count claims in docs; implementation shape is the source of truth.
- When updating phases/actions/tensor layout, update code, specs, and tests in the same change.
