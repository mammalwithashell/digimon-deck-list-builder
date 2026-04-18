# Digimon TCG Simulator

A Digimon TCG rules engine with a Gymnasium RL environment, React frontend, FastAPI hosted server, and Tauri v2 desktop app — built so RL agents can learn optimal play across the full card pool.

**Status**: pre-alpha, active development. Mid-pivot from a Python engine to a Rust engine as the source of truth.

## What This Project Does

- **Rules engine, two-track** — Rust engine (`digimon-engine/`) is the target source of truth, with PyO3 bindings (`digimon-engine-py/`) exposing it to Python as `RustHeadlessGame` (switchable via `DIGIMON_BACKEND=rust`). A transitional Python engine (`digimon_gym/engine/`) remains during the card-script migration. Both are reference-checked against the **DCGO C# client** (`DCGO/` submodule), which is the behavioral source of truth for card effects.
- **No-approximations policy** — every card effect faithfully implements all card text. No stubs, no auto-selections; every choice surfaces through `pending_selection` so the RL action space sees it and agents can learn to pick optimally. Gaps are marked BLOCKED and logged to [qa/archetype-qa/engine-gaps.md](qa/archetype-qa/engine-gaps.md) / [docs/RUST_ENGINE_GAPS.md](docs/RUST_ENGINE_GAPS.md).
- **Gymnasium RL environment** — 1375-float observation tensor, 2168 discrete actions with phase-aware masking, dense reward shaping.
- **Multiplayer PvP** — WebSocket real-time gameplay with lobby matchmaking, join codes, and spectating (with hidden-information filtering).
- **Desktop app** — Tauri v2 shell. Default build bundles a Python sidecar (engine-only, no DB/auth) for offline play against ONNX-exported AI agents. A `no-sidecar` cargo feature links the Rust engine directly into the Tauri shell for an all-Rust desktop path.
- **Admin AI pipeline** — LLM-powered card script transpilation, automated fixes, batch orchestration with safe-apply checks (hosted API only).
- **Training infrastructure** — MetaGauntlet opponent sampling, staged training pipeline, PFSP opponent weighting, deck variant generation; Q-DeckRec architect agents for deck optimization.

## Architecture

```
  ┌─────────────────────────────────────────────────────────────────┐
  │                  React Frontend (TypeScript)                     │
  │        GamePage · LobbyPage · DeckBuilder · Admin UI             │
  └──────────┬────────────────────────────────┬─────────────────────┘
             │ HTTP (local)  / invoke (Rust)  │ HTTP + WebSocket
  ┌──────────▼──────────────┐      ┌──────────▼──────────────────┐
  │  Tauri v2 Desktop Shell  │      │   Hosted FastAPI Server      │
  │  default: bundled Python │      │   api.py + routers/          │
  │    sidecar (externalBin) │      │   lobby · WS PvP · auth · DB │
  │  no-sidecar: links       │      │   admin AI pipeline          │
  │    digimon-engine direct │      └──────────┬───────────────────┘
  └──────────┬───────────────┘                 │
             └──────────────┬──────────────────┘
                            │
         ┌──────────────────┼─────────────────────────┐
         │                  │                         │
   ┌─────▼──────────┐ ┌─────▼────────────┐ ┌──────────▼──────────┐
   │ digimon_gym/    │ │  DigimonEnv       │ │  Training CLI       │
   │   engine/       │ │  (Gymnasium)      │ │  pilot_training.py  │
   │ (transitional   │ │  1375-obs         │ │  MaskablePPO        │
   │  Python)        │ │  2168-act mask    │ │  LSTM + Mask        │
   └─────┬──────────┘ └─────┬────────────┘ │  MetaGauntlet       │
         │                  │              └─────────────────────┘
         │   ┌──────────────▼──────────────┐
         └──▶│  digimon-engine-py (PyO3)   │
             │  RustHeadlessGame           │
             └──────────────┬──────────────┘
                            │
             ┌──────────────▼──────────────┐     ┌──────────────┐
             │  digimon-engine/ (Rust)     │◀────│  DCGO/       │
             │  target source of truth     │ ref │  C# submodule│
             │  game · effects · combat    │     └──────────────┘
             │  tensor · actions · cards   │
             └─────────────────────────────┘
```

## Repository Map

| Path | Purpose |
|---|---|
| `digimon-engine/` | Rust game engine (target source of truth) |
| `digimon-engine/src/game.rs` | Turn state machine, phases |
| `digimon-engine/src/effect.rs`, `effect_context.rs`, `effect_queue.rs` | Card-scripting API and triggered-effect queue |
| `digimon-engine/src/combat.rs`, `selection.rs` | Attack state machine, pending-selection / interrupt handling |
| `digimon-engine/src/tensor.rs`, `action/` | 1375-float observation + 2168-action mask/decoder |
| `digimon-engine/src/cards/`, `debug_runner.rs` | Hand-written `CardEffect` impls + deterministic test harness |
| `digimon-engine-py/` | PyO3 bindings (`RustHeadlessGame`), built via `maturin` |
| `DCGO/` | Git submodule — C# client source, behavioral reference for card effects |
| `digimon_gym/engine/` | Transitional Python engine (retired when Rust card-script migration completes) |
| `digimon_gym/engine/data/scripts/` | Python card scripts (frozen + `generated/`) |
| `digimon_gym/digimon_gym.py` | `DigimonEnv` (Gymnasium interface) |
| `digimon_gym/agents/` | RL training, wrappers, MetaGauntlet, Q-DeckRec |
| `digimon_gym/agents/maskable_recurrent/` | Custom LSTM + action-masking PPO |
| `digimon_gym/api.py`, `digimon_gym/routers/` | Hosted FastAPI app + gameplay routes |
| `digimon_gym/routers/lobby.py`, `ws_games.py`, `ws_manager.py` | Lobby, WebSocket PvP, connection manager |
| `digimon_gym/engine/state_filter.py` | Hidden-info filtering for network clients |
| `digimon_gym/engine/onnx_policy.py` | ONNX inference (no PyTorch) |
| `digimon_gym/desktop_main.py` | Desktop sidecar entry point (engine-only, no DB/auth) |
| `digimon_gym/db/`, `digimon_gym/ai/` | Hosted-API-only: auth, decks, admin AI |
| `src-tauri/` | Tauri v2 desktop shell; depends on `digimon-engine` directly and optionally bundles the Python sidecar |
| `frontend/src/` | React UI (desktop build tree-shakes admin/training via `VITE_BUILD_TARGET=desktop`) |
| `tools/export_onnx.py`, `tools/build-sidecar.sh` | Model export and desktop sidecar build |
| `qa/archetype-qa/`, `qa/qa-reports/` | Per-archetype QA, engine-gap tracker, dated gameplay reports |
| `requirements.txt`, `requirements-desktop.txt`, `requirements-training.txt` | Full hosted API / desktop sidecar / training CLI dep sets |

## Quick Start

### Prerequisites

- Python 3.11+
- Node.js 18+ (for frontend)
- Rust toolchain (for the Rust engine, PyO3 bindings, and Tauri)
- `maturin` (`pip install maturin`) if you plan to use the Rust backend from Python

### Install

Pick the dep set that matches what you're running:

```bash
pip install -r requirements.txt            # full hosted API (everything)
pip install -r requirements-desktop.txt    # desktop sidecar (engine + ONNX, no DB/torch)
pip install -r requirements-training.txt   # training CLI (engine + torch/SB3, no FastAPI/DB)
```

Initialize the DCGO submodule if you need C# reference access:

```bash
git submodule update --init --recursive
```

### Run the Hosted Backend

```bash
# Development (auto-reload)
python -m uvicorn digimon_gym.api:app --reload --reload-dir digimon_gym

# Production / long-running tasks (avoid --reload to prevent zombie watcher processes)
python -m uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000
```

### Run the Frontend

```bash
cd frontend
npm install
npm run dev
```

### Rust Engine

```bash
# Run the Rust engine test suite
cargo test --manifest-path digimon-engine/Cargo.toml

# Build + install PyO3 bindings into the active Python env
cd digimon-engine-py && maturin develop

# Run Python-side parity tests against the Rust backend
DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v
```

### Smoke-Check the RL Environment

```bash
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
# expect: (1375,) (2168,)
```

### Run Tests

```bash
python -m pytest tests -v                    # default suite (excludes AI pipeline)
python -m pytest tests/engine -v             # engine unit tests
python -m pytest tests/behavioral -v         # DebugRunner behavioral tests
python -m pytest tests/rl -v                 # RL training tests
python -m pytest tests -m scenario -v        # YAML scenario tests
python -m pytest tests/ai_pipeline -v        # AI pipeline tests (opt-in)
```

### Train an Agent

```bash
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000
```

### Export Trained Models to ONNX

```bash
python tools/export_onnx.py --type mlp  --input models/mlp_agent.zip  --output models/mlp_agent.onnx
python tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx
```

### Build the Desktop App

```bash
# Build the Python sidecar (greedy bots only, fast)
./tools/build-sidecar.sh gameplay

# Or: sidecar with auto-exported ONNX models bundled
./tools/build-sidecar.sh full

# Build the Tauri installer (default path — bundles the Python sidecar)
cd src-tauri && cargo tauri build

# All-Rust desktop path (no Python sidecar)
cd src-tauri && cargo tauri build --features no-sidecar --config tauri.rust.conf.json
```

### Run the Desktop Sidecar Standalone

```bash
python -m digimon_gym.desktop_main --port 8321 --models-dir ./models
```

## Documentation Index

Start at [docs/INDEX.md](docs/INDEX.md) for the full catalog.

| Document | Purpose |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Engineering guide for AI-assistant work in this repo |
| [AGENTS.md](AGENTS.md) | RL agent architecture, wrapper chain, gauntlet orchestration |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | API surface, RL contracts, frontend components, desktop distribution |
| [docs/TENSOR_SPEC.md](docs/TENSOR_SPEC.md) | 1375-float observation tensor layout |
| [docs/ACTION_SPEC.md](docs/ACTION_SPEC.md) | 2168-action space and phase-aware decoding |
| [docs/RULES_CONTEXT.md](docs/RULES_CONTEXT.md) | Comprehensive Digimon TCG rules reference |
| [docs/RUST_ENGINE_API.md](docs/RUST_ENGINE_API.md) | Rust scripting API: `EffectContext`, `Effect`, `CardEffect`, TDD walkthrough |
| [docs/RUST_PYTHON_PARITY.md](docs/RUST_PYTHON_PARITY.md) | Live cross-engine divergence tracker (transitional) |
| [docs/RUST_ENGINE_GAPS.md](docs/RUST_ENGINE_GAPS.md) | Rust-engine gap log from archetype audits |
| [docs/TRAINING_RUNBOOK.md](docs/TRAINING_RUNBOOK.md) | Training pipeline operations guide |
| [docs/TOOLS.md](docs/TOOLS.md) | CLI tools: transpiler, Pinecone, model export, new-set workflow |
| [docs/UI_PLAN.md](docs/UI_PLAN.md) | Frontend and API surface design |
| [docs/admin_ai_batch_runbook.md](docs/admin_ai_batch_runbook.md) | Admin AI pipeline operations |
| [docs/EDH_COMMANDER_MODE.md](docs/EDH_COMMANDER_MODE.md) | 4-player commander format spec |
| [docs/TITAN_MODE.md](docs/TITAN_MODE.md) | Asymmetric multiplayer format spec |
| [docs/plans/DESKTOP_DISTRIBUTION_PLAN.md](docs/plans/DESKTOP_DISTRIBUTION_PLAN.md) | Desktop distribution plan (WebSocket PvP, ONNX, Tauri) |
| [qa/archetype-qa/engine-gaps.md](qa/archetype-qa/engine-gaps.md) | Python-engine gap tracker |
| [qa/archetype-qa/engine-api-reference.md](qa/archetype-qa/engine-api-reference.md) | Engine scripting reference used by archetype sub-agents |

## Roadmap

**Recently added / in progress:**

- Rust engine (`digimon-engine/`) as target source of truth, with PyO3 bindings (`digimon-engine-py/`) and a `DIGIMON_BACKEND=rust` switch in `DigimonEnv`.
- TDD card-script harness in Rust (`DebugRunner`, hand-written `CardEffect` impls in `src/cards/`).
- Three-service split: desktop sidecar, hosted API, training CLI — each with its own requirements file.
- Tauri v2 desktop shell with a `no-sidecar` feature that links `digimon-engine` directly (no Python).
- Archetype-scoped AI workflows: `/implement-archetype`, `/batch-fix-cards`, `/batch-implement-cards-rust`, `/assess-archetype-rust`.
- WebSocket PvP with lobby matchmaking + spectating (hidden-info filtering).
- ONNX inference for trained agents (play against ML models without PyTorch).

**Planned but not yet implemented:**

- Complete the Rust card-script migration and retire the Python engine (tracked in [docs/RUST_PYTHON_PARITY.md](docs/RUST_PYTHON_PARITY.md)).
- **Q-DeckRec**: DQN-based deck optimization architect agent.
- **CPR (Contextual Preference Ranking)**: dense card embeddings for unseen-card recommendation.
- **Persistent meta-league**: cross-run league with ELO tracking.
- **4-player game modes**: Commander/EDH and Titan Mode (engine is currently 2-player hardcoded).

## Notes

- Avoid relying on static count claims in docs; implementation shape is the source of truth.
- When updating phases / actions / tensor layout, update code, specs, and tests in the same change.
- The Rust engine is the target source of truth — before editing engine code in either language, check [docs/RUST_PYTHON_PARITY.md](docs/RUST_PYTHON_PARITY.md) for known divergences in the area.
- Cards migrate one direction only (Python → Rust). Once a card is implemented in Rust, it is owned by Rust — do not author a new Python script for it.
