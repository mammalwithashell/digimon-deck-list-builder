# Digimon TCG Simulator

A Digimon TCG rules engine with a Gymnasium RL environment, React frontend, FastAPI hosted server, and Tauri v2 desktop app — built so RL agents can learn optimal play across the full card pool.

**Status**: pre-alpha, active development. Mid-pivot from a Python engine to a Rust engine as the source of truth.

## What This Project Does

- **Rules engine, two-track** — Rust engine (`code/digimon-engine/`) is the target source of truth, with PyO3 bindings (`code/digimon-engine-py/`) exposing it to Python as `RustHeadlessGame` (switchable via `DIGIMON_BACKEND=rust`). A sunset Python engine (`code/engine_py_legacy/engine/`) remains as reference-only during the card-script migration. Both are reference-checked against the **DCGO C# client** (`DCGO/` submodule), which is the behavioral source of truth for card effects.
- **No-approximations policy** — every card effect faithfully implements all card text. No stubs, no auto-selections; every choice surfaces through `pending_selection` so the RL action space sees it and agents can learn to pick optimally. Gaps are marked BLOCKED and logged to [qa/archetype-qa/engine-gaps.md](qa/archetype-qa/engine-gaps.md) / [docs/RUST_ENGINE_GAPS.md](docs/RUST_ENGINE_GAPS.md).
- **Gymnasium RL environment** — 1375-float observation tensor, 2192 discrete actions with phase-aware masking, dense reward shaping.
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
   │  (Rust, in-process)  │     │  server/api.py + routers/   │
   │  game · ONNX · deck  │     │  PvP / lobby / auth / DB    │
   │  tools; no DB        │     │  + /models/manifest.json    │
   └──────────────────────┘     └────────┬───────────────────┘
                                         │ PyO3 + sunset Python engine
                     ┌───────────────────┼───────────────────┐
                     │                   │                   │
              ┌──────▼───────┐ ┌─────────▼──────────┐ ┌─────────▼───────┐
              │ DigimonEnv   │ │ Python engine      │ │ Admin AI        │
              │ (Gymnasium)  │ │ (sunset reference) │ │ server/ai/      │
              │ via PyO3     │ │ engine_py_legacy/  │ │ LLM dispatch    │
              │ 1375-obs     │ │ engine/            │ │ batch orch.     │
              └──────┬───────┘ └────────────────────┘ └─────────────────┘
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

All source lives under `code/`. The repo root holds docs, infra, agent
config, and runtime data.

| Path | Purpose |
|---|---|
| `code/digimon-engine/` | Rust game engine (target source of truth) |
| `code/digimon-engine/src/game.rs` | Turn state machine, phases |
| `code/digimon-engine/src/effect.rs`, `effect_context.rs`, `effect_queue.rs` | Card-scripting API and triggered-effect queue |
| `code/digimon-engine/src/combat.rs`, `selection.rs` | Attack state machine, pending-selection / interrupt handling |
| `code/digimon-engine/src/tensor.rs`, `action/` | 1375-float observation + 2192-action mask/decoder |
| `code/digimon-engine/src/cards/`, `debug_runner.rs` | Hand-written `CardEffect` impls + deterministic test harness |
| `code/digimon-engine/src/inference/` | Rust ONNX inference (MLP + LSTM) used by the desktop app |
| `code/digimon-engine-py/` | PyO3 bindings (`RustHeadlessGame`), built via `maturin` |
| `code/digimon-dsl/` | Card-scripting DSL crate (lowering to Effect/CardEffect) |
| `DCGO/` | Git submodule — C# client source, behavioral reference for card effects |
| `code/engine_py_legacy/engine/` | Sunset Python engine (reference only; not importable from production) |
| `code/engine_py_legacy/engine/data/scripts/` | Frozen Python card scripts (one-direction migration to Rust) |
| `code/digimon_gym/digimon_gym.py` | `DigimonEnv` (Gymnasium interface) |
| `code/digimon_gym/agents/` | RL training, wrappers, MetaGauntlet, Q-DeckRec |
| `code/digimon_gym/agents/maskable_recurrent/` | Custom LSTM + action-masking PPO |
| `code/digimon_gym/inference/onnx_policy.py` | ONNX inference wrapper (hosted API / training) |
| `code/server/api.py`, `code/server/routers/` | Hosted FastAPI app + gameplay routes |
| `code/server/routers/lobby.py`, `ws_games.py`, `ws_manager.py` | Lobby, WebSocket PvP, connection manager |
| `code/engine_py_legacy/engine/state_filter.py` | Hidden-info filtering for network clients (until Rust port lands) |
| `code/server/db/`, `code/server/ai/` | Hosted-API-only: auth, decks, admin AI |
| `code/server/workers/training_worker.py`, `gauntlet_orchestrator.py` | DB-backed training queue + 3-stage orchestrator |
| `code/src-tauri/` | Tauri v2 desktop shell — Python-free; gameplay, inference, deck tools, model cache |
| `code/frontend/src/` | React UI (desktop build tree-shakes admin/training via `VITE_BUILD_TARGET=desktop`) |
| `code/tools/export_onnx.py` | SB3 → ONNX model conversion |
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
python -m uvicorn server.api:app --reload --reload-dir code/server

# Production / long-running tasks (avoid --reload to prevent zombie watcher processes)
python -m uvicorn server.api:app --host 0.0.0.0 --port 8000
```

### Run the Frontend

```bash
cd code/frontend
npm install
npm run dev
```

### Rust Engine

```bash
# Run the Rust engine test suite
cargo test --manifest-path code/digimon-engine/Cargo.toml

# Build + install PyO3 bindings into the active Python env
cd code/digimon-engine-py && maturin develop

# Run Python-side parity tests against the Rust backend
DIGIMON_BACKEND=rust python -m pytest code/tests/engine/test_rust_backend_parity.py -v
```

### Smoke-Check the RL Environment

```bash
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
# expect: (1375,) (2192,)
```

### Run Tests

```bash
python -m pytest -v                                # default suite (testpaths = code/tests)
python -m pytest code/tests/engine -v              # engine unit tests
python -m pytest code/tests/behavioral -v          # DebugRunner behavioral tests
python -m pytest code/tests/rl -v                  # RL training tests
python -m pytest -m scenario -v                    # YAML scenario tests
python -m pytest code/tests/ai_pipeline -v         # AI pipeline tests (opt-in)
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
python code/tools/export_onnx.py --type mlp  --input models/mlp_agent.zip  --output models/mlp_agent.onnx
python code/tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx
```

### Build the Desktop App

The desktop build is Python-free — gameplay, ONNX inference, and deck
tools all run inside the embedded `digimon-engine` crate via Tauri
`invoke()` commands. Trained AI models are downloaded at runtime from
the hosted API's `/models/manifest.json` into an OS-local cache.

```bash
# Prerequisites: Rust toolchain, Node.js
cd code/frontend && npm ci && npm run build -- --mode desktop
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
| [docs/ACTION_SPEC.md](docs/ACTION_SPEC.md) | 2192-action space and phase-aware decoding |
| [docs/RULES_CONTEXT.md](docs/RULES_CONTEXT.md) | Comprehensive Digimon TCG rules reference |
| [docs/RUST_ENGINE_API.md](docs/RUST_ENGINE_API.md) | Rust scripting API: `EffectContext`, `Effect`, `CardEffect`, TDD walkthrough |
| [docs/RUST_PYTHON_PARITY.md](docs/RUST_PYTHON_PARITY.md) | Live cross-engine divergence tracker (transitional) |
| [docs/RUST_ENGINE_GAPS.md](docs/RUST_ENGINE_GAPS.md) | Rust-engine gap log from archetype audits |
| [docs/TRAINING_RUNBOOK.md](docs/TRAINING_RUNBOOK.md) | Training pipeline operations guide |
| [docs/TOOLS.md](docs/TOOLS.md) | CLI tools: transpiler, Pinecone, model export, new-set workflow |
| [docs/admin_ai_batch_runbook.md](docs/admin_ai_batch_runbook.md) | Admin AI pipeline operations |
| [docs/EDH_COMMANDER_MODE.md](docs/EDH_COMMANDER_MODE.md) | 4-player commander format spec |
| [docs/TITAN_MODE.md](docs/TITAN_MODE.md) | Asymmetric multiplayer format spec |
| [qa/archetype-qa/engine-gaps.md](qa/archetype-qa/engine-gaps.md) | Python-engine gap tracker |
| [qa/archetype-qa/engine-api-reference.md](qa/archetype-qa/engine-api-reference.md) | Engine scripting reference used by archetype sub-agents |

## Roadmap

**Recently added / in progress:**

- Rust engine (`code/digimon-engine/`) as target source of truth, with PyO3 bindings (`code/digimon-engine-py/`) and a `DIGIMON_BACKEND=rust` switch in `DigimonEnv`.
- TDD card-script harness in Rust (`DebugRunner`, hand-written `CardEffect` impls in `code/digimon-engine/src/cards/`).
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
