# CLAUDE.md - AI Assistant Guide

## Scope

This document is a current-state engineering guide for working in this repository.
It focuses on stable contracts and implementation shape, not static snapshot metrics.

## Project Vision

This is a **Digimon TCG simulator** built for both human play and **RL agent training/deckbuilding**. The engine faithfully implements card effects so RL agents can learn optimal play strategies across the full card pool.

- **DCGO** (`DCGO/`) is a git submodule containing the C# source from the Digimon Card Game Online client — the **behavioral source of truth** for card effects
- **No-Approximations Policy**: every card effect must faithfully implement all card text; no stubs, no auto-selections; every choice must be exposed to the RL action space so agents can learn to make optimal decisions; gaps are marked BLOCKED and logged to `qa/archetype-qa/engine-gaps.md`

### Rust pivot (in progress)

The project is migrating to a **Rust engine as the source of truth** (`digimon-engine/`). Python is retained only for the FastAPI server (P2P games, lobby, auth) and RL training (gym/SB3); both call into the Rust engine via PyO3 bindings (`digimon-engine-py`). Card scripts are being hand-written in Rust, TDD-driven, via a forthcoming Rust-focused `batch-fix-cards` skill (analogous to the existing Python one). The no-approximations policy applies identically in Rust. `docs/RUST_PYTHON_PARITY.md` is a **transitional** tracker of cross-engine divergences — it exists only until the Python engine is retired.

## Tech Stack

- **Engine (target)**: Rust — `digimon-engine/` library crate, `digimon-engine-py/` PyO3 bindings
- **Engine (transitional)**: Python 3.11+, `digimon_gym/engine/` — retained until card-script migration completes
- **Backend**: FastAPI + Uvicorn, SQLAlchemy + PostgreSQL (hosted API only); binds to Rust engine via PyO3
- **Frontend**: React 18 + TypeScript + Vite, Zustand state management
- **Desktop**: Tauri v2 (Rust shell) — Python-free; gameplay + inference + deck tools run entirely in the embedded `digimon-engine` crate, and AI models are fetched at runtime from the hosted API's manifest
- **RL**: Gymnasium, Stable-Baselines3, ONNX Runtime for inference; env drives the Rust engine via PyO3
- **AI Pipeline**: Claude API, Pinecone vector DB, git worktrees
- **C# Reference**: DCGO submodule (`DCGO/`) — behavioral source of truth

## System Overview

The codebase is split into three deployable services:

1. **Desktop App** (`src-tauri/`) — local games vs AI agents, deck tools. No Python at runtime: gameplay, ONNX inference, and deck validation run inside the embedded `digimon-engine` crate via Tauri `invoke()` commands. Trained models are downloaded at runtime from the hosted API's `/models/manifest.json` and cached under the OS `data_dir`.
2. **Hosted API** (`digimon_gym/api.py`) — PvP WebSockets, lobby, auth, user data, recordings, admin AI, model manifest. Central server for online features.
3. **Training CLI** (`python -m digimon_gym.agents.pilot_training`) — standalone RL training. No HTTP server, no DB.

Underlying surfaces:

1. **Rust engine** (`digimon-engine/`) — rules implementation (target source of truth); exposed to Python via `digimon-engine-py` (PyO3) as `RustHeadlessGame`. Swapped into `DigimonEnv` behind `DIGIMON_BACKEND=rust`.
2. Python game engine (`digimon_gym/engine/`) — transitional; shared by all services today, retired once Rust card-script migration completes
3. RL environment and pilot training (`digimon_gym/digimon_gym.py`, `digimon_gym/agents/`)
4. React frontend (`frontend/src/`) — desktop build excludes admin/training UI via `VITE_BUILD_TARGET`
5. Tauri v2 desktop shell (`src-tauri/`) — depends on `digimon-engine` directly (no Python) for gameplay, ONNX inference, deck tools, and the model cache/downloader
6. Admin AI workflow (`digimon_gym/ai/`, `/admin/*` routes) — hosted API only

## Project Layout

```
.
├── CLAUDE.md                      # This file — project overview
├── AGENTS.md                      # RL agent architecture
├── DCGO/                          # Git submodule — DCGO C# source (reference impl)
├── digimon-engine/                # Rust game engine (target source of truth)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 # Public API re-exports
│   │   ├── game.rs                # Game struct, turn state machine, phases
│   │   ├── player.rs, permanent.rs, card_source.rs
│   │   ├── card_data.rs, card_registry.rs, cards.rs  # Metadata + effect registry
│   │   ├── effect.rs              # Effect + EffectBuilder + CardEffect trait
│   │   ├── effect_context.rs      # EffectContext — curated card-scripting API
│   │   ├── effect_queue.rs        # Triggered-effect queue + drainer
│   │   ├── modifiers.rs           # ModifierRegistry (typed + expiry)
│   │   ├── combat.rs              # Attack state machine + interrupts (Alliance/Counter/Block)
│   │   ├── selection.rs           # Pending selection / interrupt state machine
│   │   ├── tensor.rs              # Observation tensor (1375 floats, parity with Python)
│   │   ├── action/                # Action space (2168) + mask + decoder
│   │   ├── cards/test_cards.rs    # TEST-001..022 — hand-written worked examples
│   │   ├── runners/               # HeadlessRunner (RL-shaped API)
│   │   └── debug_runner.rs        # Deterministic test harness
│   └── tests/                     # Integration tests (engine_core, tensor_and_mask,
│                                  # combat_scenarios, security_effects, mask_*_parity, etc.)
├── digimon-engine-py/             # PyO3 bindings — Rust engine exposed to Python
│   ├── Cargo.toml                 # Depends on digimon-engine (path) + pyo3 + numpy
│   ├── pyproject.toml             # maturin build backend, module name "digimon_engine"
│   └── src/lib.rs                 # RustHeadlessGame class; Python player-ID convention (1/2 ↔ 0/1)
├── data/                          # Shared game data — source of truth for both engines
│   ├── cards.json                 # Full card metadata (~4085 cards)
│   ├── card_overrides.json        # Hand-maintained corrections over API ingest
│   ├── deck_library.json          # Scraped meta decklists
│   ├── archetype_aliases.json     # Canonical archetype name map
│   └── tested_cards.json          # Tested-cards allowlist (deck builder gate)
├── digimon_gym/
│   ├── data_paths.py              # Canonical paths + env overrides for data/*
│   ├── engine/                    # Headless game engine (shared by all services)
│   │   ├── game/                  # Core rules: action decoder, mask, combat, effects, tensor
│   │   ├── core/                  # Permanent, Player, CardSource
│   │   ├── data/                  # Python-side engine code + card scripts
│   │   │   ├── card_database.py, card_registry.py, tensor_layout.py, ...
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
│   └── digimon_gym.py             # DigimonEnv (Gymnasium)
├── frontend/src/
│   ├── pages/                     # GamePage, LobbyPage, DeckBuilderPage, Admin*
│   ├── components/board/          # GameBoard, HandZone, BattleArea, MemoryGauge
│   ├── components/game/           # ActionBar, overlays, selection UI
│   ├── api/                       # REST + WebSocket clients
│   └── App.tsx                    # Route map
├── src-tauri/                     # Tauri v2 desktop shell — Rust-only, hosts
│   │                              # gameplay, ONNX inference, deck tools, and
│   │                              # the runtime-downloaded model cache
│   └── src/
│       ├── engine_commands.rs     # `rust_create_game` / step / submit + agent loop
│       ├── inference_state.rs     # ONNX session cache per model_id
│       ├── models.rs              # Manifest fetch + SHA-verified download cache
│       └── deck_commands.rs       # parse / validate / tested-cards Tauri wrappers
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

**Engine-only routers** (hosted API, no DB imports — mirrored by Rust Tauri
commands on desktop):
- `health`, `games`, `deck_tools`, `simulations`, `replays`

**DB/auth-required routers** (hosted API only):
- `lobby`, `ws_games`, `recordings`, all `db/routers/*`

**Standalone agent modules** (no DB, no HTTP — training CLI):
- `pilot_training`, `gauntlet`, `deck_pool`, `features_extractor`, `maskable_recurrent/`

**DB-dependent agent modules** (hosted API only):
- `training_worker`, `gauntlet_orchestrator`

**Requirements files:**
- `requirements.txt` — full hosted API (all deps)
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

# Tauri desktop app (requires Rust toolchain; Python-free at runtime)
cd src-tauri && cargo tauri dev                     # development
cd src-tauri && cargo tauri build                   # production installers
cargo test --manifest-path src-tauri/Cargo.toml     # Tauri-layer unit tests

# Rust engine tests
cargo test --manifest-path digimon-engine/Cargo.toml
cargo test --manifest-path digimon-engine/Cargo.toml --test security_effects
cargo test --manifest-path digimon-engine/Cargo.toml --test test_cards_behavioral

# PyO3 bindings (build + install into active Python env)
cd digimon-engine-py && maturin develop

# Python-side Rust-backend parity test (uses Rust engine via env var)
DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v
```

## Working Rules

1. Keep tensor and action specs in sync with `game.py` and frontend constants.
2. Preserve headless engine behavior; UI reflects state, it does not own rules.
3. Do not bypass action masking in agent logic.
4. When updating phases/actions, update tests and both spec docs in the same change.
5. Keep docs stable: avoid stale hardcoded snapshot claims unless explicitly time-stamped.
6. When threading LSTM state during evaluation/inference, reset state to `None` at episode boundaries.
7. OpponentWrapper discards dense rewards from opponent steps; only terminal rewards pass through.
8. The desktop Tauri build must not link any Python runtime. All gameplay, inference, and deck tooling dispatch through Tauri `invoke()` into `digimon-engine`; trained models are downloaded at runtime via `src-tauri/src/models.rs` and cached under `dirs::data_dir()/digimon-tcg/models/`.
9. WebSocket state broadcasts must use `state_filter.py` — never send raw `to_ui_json()` to network clients.
10. ONNX policies must call `reset()` at episode boundaries for LSTM models (same rule as SB3 LSTM state threading).
11. Engine-only routers (`games.py`, `replays.py`, `simulations.py`, `deck_tools.py`) must not import from `digimon_gym.db.*` or `digimon_gym.ai.*`.
12. Training CLI modules (`pilot_training.py`, `gauntlet.py`, `deck_pool.py`) must not import from `digimon_gym.db.*`.
13. Desktop frontend builds use `VITE_BUILD_TARGET=desktop` to tree-shake admin/training UI.
14. `state_filter.py` must redact both `handIds` and `handCards` for opponents — never leak card metadata.
15. Game animation components (`DigivolveBanner`, `BattleEffect`) subscribe to `store.events` and track `lastSeqRef` to avoid replaying stale events.
16. `Game.surrender()` must emit the `surrender` event before calling `declare_winner()` so event listeners see the surrender before game_over.
17. The no-approximations policy applies identically to the Rust engine — no stubs, no auto-selections; every choice must surface through `pending_selection` so the RL action space sees it.
18. New Rust card effects are TDD: write a failing behavioral test under `digimon-engine/tests/` using `DebugRunner` (see `tests/test_cards_behavioral.rs`) before implementing the `CardEffect` struct.
19. Before editing engine code in either language, check `docs/RUST_PYTHON_PARITY.md` for known divergences in the area — it's the authoritative cross-engine state.
20. `digimon-engine-py/src/lib.rs` must preserve the Python player-ID convention (Python 1/2 ↔ Rust 0/1 translation at the binding boundary); callers on both sides depend on it.
21. Do not author new Python-side card scripts for cards already implemented in Rust — cards migrate one direction only (Python → Rust) and are then owned by Rust.

## Documentation

Detailed reference docs live in `docs/` — see [docs/INDEX.md](docs/INDEX.md) for the full list.

Key references:
- **Architecture**: `docs/ARCHITECTURE.md` — API surface, RL contracts, frontend components, desktop distribution
- **Spec contracts**: `docs/TENSOR_SPEC.md` (obs tensor), `docs/ACTION_SPEC.md` (action space)
- **Tools**: `docs/TOOLS.md` — card pipeline, transpiler, Pinecone, model export, new-set workflow
- **Training**: `docs/TRAINING_RUNBOOK.md` + `AGENTS.md` (wrapper chain, gauntlet, pipeline)
- **Rules**: `docs/RULES_CONTEXT.md` — official Digimon TCG rules reference

**Rust engine (target source of truth — read all three before editing engine code):**
- **Design / phase plan**: `docs/superpowers/specs/2026-04-15-rust-engine-rewrite-design.md` — north-star architecture, crate structure, phase roadmap
- **Scripting API**: `docs/RUST_ENGINE_API.md` — `EffectContext` API, `Effect` builder, `CardEffect` trait, TDD walkthrough for new cards
- **Cross-engine parity**: `docs/RUST_PYTHON_PARITY.md` — live divergence tracker and per-phase progress; transitional, retired when the Python engine is

**Rust card scripting**: a Rust-focused `batch-fix-cards` skill (analogous to the existing Python `/batch-fix-cards` but targeting `digimon-engine/src/cards/` + `digimon-engine/tests/`) is planned for the forthcoming card-migration phase. Until then, author new Rust card effects directly per the TDD walkthrough in `RUST_ENGINE_API.md`.

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
