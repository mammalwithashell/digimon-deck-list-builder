# Digimon TCG Simulator

A Digimon TCG rules engine with a Gymnasium RL environment, React frontend, FastAPI hosted server, and Tauri v2 desktop app — built so RL agents can learn optimal play across the full card pool.

**Status**: pre-alpha, active development. Mid-pivot from a Python engine to a Rust engine as the source of truth.

## What This Project Does

- **Rules engine, two-track** — Rust engine (`digimon-engine/`) is the target source of truth, with PyO3 bindings (`digimon-engine-py/`) exposing it to Python as `RustHeadlessGame` (switchable via `DIGIMON_BACKEND=rust`). A transitional Python engine (`digimon_gym/engine/`) remains during the card-script migration. Both are reference-checked against the **DCGO C# client** (`DCGO/` submodule), which is the behavioral source of truth for card effects.
- **No-approximations policy** — every card effect faithfully implements all card text. No stubs, no auto-selections; every choice surfaces through `pending_selection` so the RL action space sees it and agents can learn to pick optimally. Gaps are marked BLOCKED and logged to [qa/archetype-qa/engine-gaps.md](qa/archetype-qa/engine-gaps.md) / [docs/RUST_ENGINE_GAPS.md](docs/RUST_ENGINE_GAPS.md).
- **Gymnasium RL environment** — 1375-float observation tensor, 2168 discrete actions with phase-aware masking, dense reward shaping.
- **Multiplayer PvP** — WebSocket real-time gameplay with lobby matchmaking, join codes, and spectating (with hidden-information filtering).
- **Desktop app** — Tauri v2 shell with the embedded Rust engine (no Python at runtime); ONNX models downloaded at runtime from the hosted API's manifest.
- **Admin AI pipeline** — LLM-powered card script transpilation, automated fixes, batch orchestration with safe-apply checks (hosted API only).
- **Training infrastructure** — MetaGauntlet opponent sampling, staged training pipeline, PFSP opponent weighting, deck variant generation; Q-DeckRec architect agents for deck optimization.

## Architecture

```
   ┌───────────────────────────────────────────────────────────┐
   │              Tauri v2 Desktop Shell (Rust)                 │
   │   Links digimon-engine statically — no Python runtime      │
   │   ONNX models downloaded from hosted API manifest          │
   └──────────────────────┬────────────────────────────────────┘
                          │ Tauri invoke()
   ┌──────────────────────▼────────────────────────────────────┐
   │         React Frontend (TypeScript)                        │
   │   GamePage · DeckBuilder · ModelsPage (desktop only)      │
   └─────────┬──────────────────────────┬──────────────────────┘
   invoke()  │ (desktop)                 │ WebSocket / HTTPS (web)
   ┌─────────▼────────────┐     ┌────────▼───────────────────┐
   │  digimon-engine      │     │  FastAPI Server (Python)    │
   │  (Rust, in-process)  │     │  api.py + routers/          │
   │  game · ONNX · deck  │     │  PvP / lobby / auth / DB    │
   │  tools; no DB        │     │  + /models/manifest.json    │
   └──────────────────────┘     └────────┬───────────────────┘
                                         │ PyO3 + transitional Python engine
                     ┌───────────────────┼───────────────────┐
                     │                   │                   │
              ┌──────▼───────┐ ┌─────────▼──────┐ ┌─────────▼───────┐
              │ DigimonEnv   │ │ Python engine  │ │ Admin AI        │
              │ (Gymnasium)  │ │ (transitional) │ │ ai/             │
              │ via PyO3     │ │ digimon_gym/   │ │ LLM dispatch    │
              │ 1375-obs     │ │ engine/        │ │ batch orch.     │
              └──────┬───────┘ └────────────────┘ └─────────────────┘
                     │
            ┌────────▼────────┐
            │  RL Agents      │
            │  MaskablePPO    │   ┌──────────────┐
            │  LSTM + Mask    │   │  DCGO/       │
            │  MetaGauntlet   │   │  C# submod   │ ← behavioral reference
            └─────────────────┘   │  for cards   │   for card scripting
                                  └──────────────┘
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
| `digimon-engine/src/inference/` | Rust ONNX inference (MLP + LSTM) used by the desktop app |
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
| `digimon_gym/engine/onnx_policy.py` | ONNX inference wrapper (hosted API / training) |
| `digimon_gym/db/`, `digimon_gym/ai/` | Hosted-API-only: auth, decks, admin AI |
| `src-tauri/` | Tauri v2 desktop shell — Python-free; gameplay, inference, deck tools, model cache |
| `frontend/src/` | React UI (desktop build tree-shakes admin/training via `VITE_BUILD_TARGET=desktop`) |
| `tools/export_onnx.py` | SB3 → ONNX model conversion |
| `qa/archetype-qa/`, `qa/qa-reports/` | Per-archetype QA, engine-gap tracker, dated gameplay reports |
| `requirements.txt`, `requirements-training.txt` | Full hosted API / training CLI dep sets |

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

The desktop build is Python-free — gameplay, ONNX inference, and deck
tools all run inside the embedded `digimon-engine` crate via Tauri
`invoke()` commands. Trained AI models are downloaded at runtime from
the hosted API's `/models/manifest.json` into an OS-local cache.

```bash
# Prerequisites: Rust toolchain, Node.js
cd frontend && npm ci && npm run build -- --mode desktop
cd ../src-tauri && cargo tauri build
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
- Three-service split: desktop, hosted API, training CLI — each with its own requirements file.
- Python-free Tauri v2 desktop shell — `digimon-engine` embedded directly, ONNX models cached from the hosted API manifest.
- Archetype-scoped AI workflows: `/implement-archetype`, `/batch-fix-cards`, `/batch-implement-cards-rust`, `/assess-archetype-rust`.
- WebSocket PvP with lobby matchmaking + spectating (hidden-info filtering).

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
