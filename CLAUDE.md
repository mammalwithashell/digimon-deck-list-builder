# CLAUDE.md - AI Assistant Guide

## Scope

This document is a current-state engineering guide for working in this repository.
It focuses on stable contracts and implementation shape, not static snapshot metrics.

## Project Vision

This is a **Digimon TCG simulator** built for both human play and **RL agent training/deckbuilding**. The engine faithfully implements card effects so RL agents can learn optimal play strategies across the full card pool.

- **DCGO** (`DCGO/`) is a git submodule containing the C# source from the Digimon Card Game Online client — the **behavioral source of truth** for card effects
- **No-Approximations Policy**: every card effect must faithfully implement all card text; no stubs, no auto-selections; every choice must be exposed to the RL action space so agents can learn to make optimal decisions; gaps are marked BLOCKED and logged to `qa/archetype-qa/engine-gaps.md`

## Tech Stack

- **Engine**: Python 3.11+, headless game engine (`digimon_gym/engine/`)
- **Backend**: FastAPI + Uvicorn, SQLAlchemy + PostgreSQL (hosted API only)
- **Frontend**: React 18 + TypeScript + Vite, Zustand state management
- **Desktop**: Tauri v2 (Rust shell) + PyInstaller sidecar
- **RL**: Gymnasium, Stable-Baselines3, ONNX Runtime for inference
- **AI Pipeline**: Claude API, Pinecone vector DB, git worktrees
- **C# Reference**: DCGO submodule (`DCGO/`) — behavioral source of truth

## System Overview

The codebase is split into three deployable services sharing a common engine:

1. **Desktop Sidecar** (`digimon_gym/desktop_main.py`) — local games vs AI agents, deck tools, simulations, replays. No DB, no auth. Bundled as a Tauri v2 desktop app.
2. **Hosted API** (`digimon_gym/api.py`) — PvP WebSockets, lobby, auth, user data, recordings, admin AI. Central server for online features.
3. **Training CLI** (`python -m digimon_gym.agents.pilot_training`) — standalone RL training. No HTTP server, no DB.

Underlying surfaces:

1. Headless Digimon game engine (`digimon_gym/engine/`) — shared by all services
2. RL environment and pilot training (`digimon_gym/digimon_gym.py`, `digimon_gym/agents/`)
3. React frontend (`frontend/src/`) — desktop build excludes admin/training UI via `VITE_BUILD_TARGET`
4. Tauri v2 desktop shell (`src-tauri/`)
5. Admin AI workflow (`digimon_gym/ai/`, `/admin/*` routes) — hosted API only

## Project Layout

```
.
├── CLAUDE.md                      # This file — project overview
├── AGENTS.md                      # RL agent architecture
├── DCGO/                          # Git submodule — DCGO C# source (reference impl)
├── digimon_gym/
│   ├── engine/                    # Headless game engine (shared by all services)
│   │   ├── game/                  # Core rules: action decoder, mask, combat, effects, tensor
│   │   ├── core/                  # Permanent, Player, CardSource
│   │   ├── data/                  # cards.json, scripts/, card_registry, tensor_layout
│   │   │   └── scripts/           # Card effect scripts (frozen + generated/)
│   │   ├── interfaces/            # CardEffect, Modifiers
│   │   ├── validation/            # Digivolve validator, play validators
│   │   ├── runners/               # Game runner variants
│   │   ├── state_filter.py        # Hidden info filtering for network play
│   │   └── onnx_policy.py         # ONNX inference (no PyTorch)
│   ├── agents/                    # RL training modules
│   │   ├── pilot_training.py      # MLP/LSTM training entrypoint
│   │   ├── gauntlet.py            # MetaGauntlet opponent sampling
│   │   ├── maskable_recurrent/    # Custom recurrent+mask PPO
│   │   └── architect_*.py         # Q-DeckRec deck optimization agents
│   ├── routers/                   # FastAPI routers (games, lobby, ws, replays, etc.)
│   ├── db/                        # SQLAlchemy models, auth, DB routers (hosted API only)
│   ├── ai/                        # Admin AI pipeline (hosted API only)
│   ├── api.py                     # Hosted API app assembly
│   ├── desktop_main.py            # Desktop sidecar entry point (no DB/auth)
│   └── digimon_gym.py             # DigimonEnv (Gymnasium)
├── frontend/src/
│   ├── pages/                     # GamePage, LobbyPage, DeckBuilderPage, Admin*
│   ├── components/board/          # GameBoard, HandZone, BattleArea, MemoryGauge
│   ├── components/game/           # ActionBar, overlays, selection UI
│   ├── api/                       # REST + WebSocket clients
│   └── App.tsx                    # Route map
├── src-tauri/                     # Tauri v2 desktop shell (Rust)
├── tools/                         # CLI tools (see docs/TOOLS.md)
│   ├── transpiler/                # C#→Python transpiler package
│   └── archive/                   # One-time migration scripts
├── docs/                          # Project documentation
│   ├── INDEX.md                   # Documentation index
│   ├── ARCHITECTURE.md            # Detailed architecture reference
│   ├── TENSOR_SPEC.md, ACTION_SPEC.md, TRAINING_RUNBOOK.md, ...
│   └── TOOLS.md                   # CLI tools reference
├── qa/
│   ├── archetype-qa/              # Per-archetype QA, engine API ref, engine gaps
│   └── qa-reports/                # Gameplay QA reports, validated cards index
└── tests/
    ├── conftest.py                # Shared fixtures (reset_registry, debug_runner)
    ├── helpers/                   # Test utilities (make_card, GameBuilder)
    ├── engine/                    # Engine unit tests (tensor, actions, keywords, timing)
    ├── runners/                   # Game runner tests (headless, interactive, replay)
    ├── behavioral/                # DebugRunner behavioral tests (real card effects)
    ├── rl/                        # RL training tests (gauntlet, LSTM, workers)
    ├── api/                       # Hosted API tests (DB, auth)
    ├── ai_pipeline/               # AI pipeline tests (excluded from default runs)
    └── scenarios/                 # YAML scenario files (auto-discovered by pytest)
```

## Service Boundaries

**Engine-only routers** (safe for desktop sidecar — no DB imports):
- `health`, `games`, `deck_tools`, `simulations`, `replays`

**DB/auth-required routers** (hosted API only):
- `lobby`, `ws_games`, `recordings`, all `db/routers/*`

**Standalone agent modules** (no DB, no HTTP — training CLI):
- `pilot_training`, `gauntlet`, `deck_pool`, `features_extractor`, `maskable_recurrent/`

**DB-dependent agent modules** (hosted API only):
- `training_worker`, `gauntlet_orchestrator`

**Requirements files:**
- `requirements.txt` — full hosted API (all deps)
- `requirements-desktop.txt` — sidecar (engine + ONNX, no DB/torch)
- `requirements-training.txt` — training CLI (engine + torch/SB3, no FastAPI/DB)

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

# Tests (default run excludes AI pipeline tests)
python -m pytest tests -v

# By subdirectory
python -m pytest tests/engine -v                       # Engine unit tests
python -m pytest tests/behavioral -v                   # DebugRunner behavioral tests
python -m pytest tests/runners -v                      # Game runner tests
python -m pytest tests/rl -v                           # RL training tests

# By marker
python -m pytest tests -m scenario -v                  # YAML scenario tests only
python -m pytest tests/ai_pipeline -v                   # AI pipeline tests (opt-in)
python -m pytest tests -m "not slow" -v                # Skip slow smoke tests

# RL training
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000

# Env smoke check
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"

# ONNX model export (requires PyTorch)
python tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx
python tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx

# Desktop sidecar build
./tools/build-sidecar.sh gameplay   # greedy bots only
./tools/build-sidecar.sh full       # auto-exports ONNX + bundles models

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
11. Engine-only routers (`games.py`, `replays.py`, `simulations.py`, `deck_tools.py`) must not import from `digimon_gym.db.*` or `digimon_gym.ai.*`.
12. Training CLI modules (`pilot_training.py`, `gauntlet.py`, `deck_pool.py`) must not import from `digimon_gym.db.*`.
13. Desktop frontend builds use `VITE_BUILD_TARGET=desktop` to tree-shake admin/training UI.
14. `state_filter.py` must redact both `handIds` and `handCards` for opponents — never leak card metadata.
15. Game animation components (`DigivolveBanner`, `BattleEffect`) subscribe to `store.events` and track `lastSeqRef` to avoid replaying stale events.
16. `Game.surrender()` must emit the `surrender` event before calling `declare_winner()` so event listeners see the surrender before game_over.

## Documentation

Detailed reference docs live in `docs/` — see [docs/INDEX.md](docs/INDEX.md) for the full list.

Key references:
- **Architecture**: `docs/ARCHITECTURE.md` — API surface, RL contracts, frontend components, desktop distribution
- **Spec contracts**: `docs/TENSOR_SPEC.md` (obs tensor), `docs/ACTION_SPEC.md` (action space)
- **Tools**: `docs/TOOLS.md` — card pipeline, transpiler, Pinecone, model export, new-set workflow
- **Training**: `docs/TRAINING_RUNBOOK.md` + `AGENTS.md` (wrapper chain, gauntlet, pipeline)
- **Rules**: `docs/RULES_CONTEXT.md` — official Digimon TCG rules reference

## QA Artifacts

- `qa/archetype-qa/` — per-archetype implementation QA, engine API reference, engine gaps tracker
- `qa/qa-reports/` — dated gameplay test reports, validated cards index

## Pinecone MCP Integration

The `/implement-archetype` skill uses Pinecone (`digimon-engine` index) for sub-agent retrieval. MCP server: `@pinecone-database/mcp` in `.mcp.json`. Requires `PINECONE_API_KEY` env var.

| Namespace | Content | ~Vectors |
|-----------|---------|----------|
| `engine-api` | Engine API reference + decomposed engine source | ~300 |
| `card-scripts` | Python scripts (frozen + generated) + C# reference | ~6,000 |
| `card-metadata` | Per-card entries from cards.json | ~4,000 |
| `rules-docs` | RULES_CONTEXT.md, ACTION_SPEC.md, TENSOR_SPEC.md, engine-gaps.md | ~100 |

See `docs/TOOLS.md` §5 for ingestion and verification commands.
