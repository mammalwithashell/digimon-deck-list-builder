# CLAUDE.md - AI Assistant Guide

## Scope

This document is a current-state engineering guide for working in this repository.
It focuses on stable contracts and implementation shape, not static snapshot metrics.

## Project Vision

This is a **Digimon TCG simulator** built for both human play and **RL agent training/deckbuilding**. The engine faithfully implements card effects so RL agents can learn optimal play strategies across the full card pool.

- **No-Approximations Policy**: every card effect must faithfully implement all card text; no stubs, no auto-selections; every choice must be exposed to the RL action space so agents can learn to make optimal decisions; gaps are marked BLOCKED and logged to `qa/archetype-qa/engine-gaps.md`

### Source priority for card / keyword / rules questions

When questioning *what a card or keyword should do* — for instance, "is Fragment optional?", "what does Save target?", "does Progress block X?" — consult sources in this order. **Do not skip ahead to DCGO when a primary source could answer the question.**

1. **Printed card text** — `data/cards.json` (`effect_text`, `inherited_text`, `security_text`). The text on the physical card is what the no-approximations policy obligates us to implement. For an unfamiliar card, check the entry by `card_id` first.
2. **Comprehensive Rules Manual** — `docs/RULES_CONTEXT.md` (decomposed) and `Digimon TCG resources/general_rule.pdf` (canonical). Keyword semantics live in §16; rule numbers like `16-36` cite the manual directly. **`RULES_CONTEXT.md` is the canonical interpretation of every keyword and timing — check it before reaching for an implementation reference.**
3. **Fandom wiki** — `https://digimoncardgame.fandom.com/wiki/<CARD-ID>` for card-specific text + community-curated ruling notes (e.g. quirky interactions, errata). Useful when the printed text is terse.
4. **DCGO C# source** (`DCGO/`) — the behavioral *implementation* reference. Use this as a tiebreaker for ambiguous behavior the primary sources don't pin down (e.g., the exact processing order of an interaction, the inner UI flow of a multi-pick). DCGO is not authoritative on whether something is optional or mandatory — that's printed text and rules manual territory. A C# flag like `isOptional=true` in `SetUpActivateClass` reflects DCGO's reading of the card; if the printed text and rules disagree, the printed text wins.

DCGO remains useful — it captures every implementation detail the rules text glosses over — but its role is **after** the primary sources, not instead of them. Cross-checking against printed text / rules first prevents the misreading-DCGO-internals failure mode where a parameter name (`canNoSelect: () => false`) gets read as the cardinal answer when in fact it governs a sub-prompt within an already-optional flow.

### Rust pivot (in progress)

The project is migrating to a **Rust engine as the source of truth** (`code/digimon-engine/`). Python is retained only for the FastAPI server (P2P games, lobby, auth) and RL training (gym/SB3); both call into the Rust engine via PyO3 bindings (`code/digimon-engine-py/`). Card scripts are being hand-written in Rust, TDD-driven, via a forthcoming Rust-focused `batch-fix-cards` skill (analogous to the existing Python one). The no-approximations policy applies identically in Rust. `docs/RUST_PYTHON_PARITY.md` is a **transitional** tracker of cross-engine divergences — it exists only until the Python engine is retired.

## Tech Stack

- **Engine (target)**: Rust — `code/digimon-engine/` library crate, `code/digimon-engine-py/` PyO3 bindings
- **Engine (sunset)**: Python 3.11+, `code/engine_py_legacy/engine/` — reference material only; not importable from production code
- **Backend**: FastAPI + Uvicorn, SQLAlchemy + PostgreSQL (hosted API only); binds to Rust engine via PyO3
- **Frontend**: React 18 + TypeScript + Vite, Zustand state management
- **Desktop**: Tauri v2 (Rust shell) — Python-free; gameplay + inference + deck tools run entirely in the embedded `digimon-engine` crate, and AI models are fetched at runtime from the hosted API's manifest
- **RL**: Gymnasium, Stable-Baselines3, ONNX Runtime for inference; env drives the Rust engine via PyO3
- **AI Pipeline**: Claude API, Pinecone vector DB, git worktrees
- **C# Reference**: DCGO submodule (`DCGO/`) — implementation reference for behavioral details. **Not** the canonical source for card / keyword / rules questions; see "Source priority" above (printed text + Rules Manual + fandom wiki come first).

## System Overview

The codebase is split into three deployable services:

1. **Desktop App** (`code/src-tauri/`) — local games vs AI agents, deck tools. No Python at runtime: gameplay, ONNX inference, and deck validation run inside the embedded `digimon-engine` crate via Tauri `invoke()` commands. Trained models are downloaded at runtime from the hosted API's `/models/manifest.json` and cached under the OS `data_dir`.
2. **Hosted API** (`code/server/api.py`) — PvP WebSockets, lobby, auth, user data, recordings, admin AI, model manifest. Central server for online features.
3. **Training CLI** (`python -m digimon_gym.agents.pilot_training`) — standalone RL training. No HTTP server, no DB.

Underlying surfaces:

1. **Rust engine** (`code/digimon-engine/`) — rules implementation (target source of truth); exposed to Python via `code/digimon-engine-py/` (PyO3) as `RustHeadlessGame`. Swapped into `DigimonEnv` behind `DIGIMON_BACKEND=rust`.
2. Python legacy engine (`code/engine_py_legacy/engine/`) — sunset reference; not importable from production code, retired once Rust card-script migration completes
3. RL environment and pilot training (`code/digimon_gym/digimon_gym.py`, `code/digimon_gym/agents/`)
4. React frontend (`code/frontend/src/`) — desktop build excludes admin/training UI via `VITE_BUILD_TARGET`
5. Tauri v2 desktop shell (`code/src-tauri/`) — depends on `digimon-engine` directly (no Python) for gameplay, ONNX inference, deck tools, and the model cache/downloader
6. Admin AI workflow (`code/server/ai/`, `/admin/*` routes) — hosted API only

## Project Layout

All source lives under `code/`. The repo root holds only docs, infra,
agent config, runtime data, and project-level configs.

```
.
├── CLAUDE.md                      # This file — project overview
├── AGENTS.md                      # RL agent architecture
├── README.md, GEMINI.md
├── Cargo.toml                     # Rust workspace (members live under code/)
├── pyproject.toml                 # Packages server + digimon_gym from code/
├── requirements.txt, requirements-server.txt, requirements-training.txt
├── alembic/, alembic.ini          # DB migrations (hosted API only)
├── Dockerfile, Dockerfile.training, docker-compose*.yml, Caddyfile
├── .github/                       # CI workflows
├── .claude/                       # Agent skills + worktrees
├── .codex/                        # Codex skills, including Rust DSL archetype readiness assessment
├── DCGO/                          # Git submodule — DCGO C# source (behavioral reference)
├── data/                          # Shared game data — source of truth for both engines
│   ├── cards.json                 # Full card metadata (~4085 cards)
│   ├── card_overrides.json        # Hand-maintained corrections over API ingest
│   ├── deck_library.json          # Scraped meta decklists
│   ├── archetype_aliases.json     # Canonical archetype name map
│   └── tested_cards.json          # Tested-cards allowlist (deck builder gate)
├── docs/                          # Project documentation
│   ├── INDEX.md                   # Documentation index
│   ├── ARCHITECTURE.md            # Detailed architecture reference
│   ├── TENSOR_SPEC.md, ACTION_SPEC.md, TRAINING_RUNBOOK.md, ...
│   └── TOOLS.md                   # CLI tools reference
├── qa/
│   ├── archetype-qa/              # Per-archetype QA, engine API ref, engine gaps
│   └── qa-reports/                # Gameplay QA reports, validated cards index
├── ops/, scripts/                 # Deploy + operational scripts
├── training_jobs/                 # On-disk RL training run artifacts
├── models/                        # Trained-model output dir (gitignored, scanned by /models/manifest.json)
└── code/                          # All source lives here
    ├── data_paths.py              # Canonical paths + env overrides for data/*
    ├── digimon-engine/            # Rust game engine (target source of truth)
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
    │   │   ├── action/                # Action space (2192) + mask + decoder
    │   │   ├── cards/test_cards.rs    # TEST-001..022 — hand-written worked examples
    │   │   ├── runners/               # HeadlessRunner (RL-shaped API)
    │   │   └── debug_runner.rs        # Deterministic test harness
    │   └── tests/                     # Integration tests (engine_core, tensor_and_mask,
    │                                  # combat_scenarios, security_effects, mask_*_parity, etc.)
    ├── digimon-engine-py/         # PyO3 bindings — Rust engine exposed to Python
    │   ├── Cargo.toml             # Depends on digimon-engine (path) + pyo3 + numpy
    │   ├── pyproject.toml         # maturin build backend, module name "digimon_engine"
    │   └── src/lib.rs             # RustHeadlessGame class; Python player-ID convention (1/2 ↔ 0/1)
    ├── digimon-dsl/               # Card-scripting DSL crate (lowering to Effect/CardEffect)
    ├── engine_py_legacy/          # Sunset Python engine — reference material only
    │   ├── engine/                # Headless engine: game/, core/, data/, runners/, validation/, ...
    │   │   └── data/scripts/      # Frozen Python card scripts (one-direction migration to Rust)
    │   └── tests/                 # Legacy Python tests (excluded from default pytest collection)
    ├── digimon_gym/               # RL only — no FastAPI, no DB
    │   ├── digimon_gym.py         # DigimonEnv (Gymnasium)
    │   ├── agents/                # RL training modules
    │   │   ├── pilot_training.py  # MLP/LSTM training entrypoint
    │   │   ├── gauntlet.py        # MetaGauntlet opponent sampling
    │   │   ├── deck_pool.py, league_wrapper.py, training_metrics.py
    │   │   ├── maskable_recurrent/   # Custom recurrent+mask PPO
    │   │   └── architect_*.py     # Q-DeckRec deck optimization agents
    │   └── inference/onnx_policy.py  # ONNX inference (no PyTorch)
    ├── server/                    # Hosted API (FastAPI) — DB-aware code
    │   ├── api.py                 # App assembly + router registration
    │   ├── env.py                 # Env-var resolution
    │   ├── digilab_client.py      # DigiLab DB client
    │   ├── routers/               # FastAPI routers (games, lobby, ws, replays, deck_tools, ...)
    │   ├── db/                    # SQLAlchemy models, auth, DB routers
    │   ├── ai/                    # Admin AI pipeline (hosted API only)
    │   ├── classifier/            # Issue/task classifier
    │   ├── storage/               # Object/file storage adapters
    │   └── workers/               # training_worker.py, gauntlet_orchestrator.py
    ├── frontend/src/
    │   ├── pages/                 # GamePage, LobbyPage, DeckBuilderPage, Admin*
    │   ├── components/board/      # GameBoard, HandZone, BattleArea, MemoryGauge
    │   ├── components/game/       # ActionBar, overlays, selection UI
    │   ├── api/                   # REST + WebSocket clients
    │   └── App.tsx                # Route map
    ├── src-tauri/                 # Tauri v2 desktop shell — Rust-only; hosts gameplay,
    │   │                          # ONNX inference, deck tools, and the model cache
    │   └── src/
    │       ├── engine_commands.rs     # `rust_create_game` / step / submit + agent loop
    │       ├── inference_state.rs     # ONNX session cache per model_id
    │       ├── models.rs              # Manifest fetch + SHA-verified download cache
    │       └── deck_commands.rs       # parse / validate / tested-cards Tauri wrappers
    ├── tools/                     # CLI tools (see docs/TOOLS.md)
    │   ├── archive/               # One-time migration scripts
    │   ├── dsl-schema-export/     # DSL JSON-schema generator (Cargo workspace member)
    │   └── dsl-lint/              # DSL linter (Cargo workspace member)
    └── tests/                     # Default pytest tree (testpaths = code/tests)
        ├── conftest.py            # Shared fixtures (reset_registry, debug_runner)
        ├── helpers/               # Test utilities (make_card, GameBuilder)
        ├── engine/                # Engine unit tests (tensor, actions, keywords, timing)
        ├── runners/               # Game runner tests (headless, interactive, replay)
        ├── behavioral/            # DebugRunner behavioral tests (real card effects)
        ├── rl/                    # RL training tests (gauntlet, LSTM, workers)
        ├── api/                   # Hosted API tests (DB, auth)
        ├── ai_pipeline/           # AI pipeline tests (excluded from default runs)
        └── scenarios/             # YAML scenario files (auto-discovered by pytest)
```

**Python imports unchanged.** `pip install -e .` packages `server` and
`digimon_gym` from `code/`; `pyproject.toml`'s `pythonpath = ["code"]`
lets pytest resolve them. Dotted module names like `from digimon_gym.X
import Y` and `from server.X import Y` work identically — only
filesystem paths changed.

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

**Read-only operator MCPs** (local + dev only — no DB, no HTTP):
- `digimon-engine-mcp` (Rust binary, `code/digimon-engine-mcp/`) — per-game engine forensics over `LiveGame` and recordings.
- `digimon-training-mcp` (Python package, `code/digimon-training-mcp/`) — cross-game / time-series inspection of `runs/` + `models/` filesystem artifacts. Does not import from `server.*`, `digimon_gym.db.*`, or any binding crate.

**Requirements files:**
- `requirements.txt` — full hosted API (all deps)
- `requirements-training.txt` — training CLI (engine + torch/SB3, no FastAPI/DB)
- `requirements-mcp.txt` — training-inspection MCP (`mcp` SDK + `tensorboard`, kept out of the training CLI's lean dep set)

## Commands

Run from repo root unless noted.

```bash
# Install (editable; packages server + digimon_gym from code/)
pip install -r requirements.txt
pip install -e .

# Backend API (development)
python -m uvicorn server.api:app --reload --reload-dir code/server

# Backend API (production / long-running tasks)
# NOTE: Do NOT use --reload for long-running tasks (creates zombie processes)
python -m uvicorn server.api:app --host 0.0.0.0 --port 8000

# Frontend
cd code/frontend
npm install
npm run dev

# Tests (default run excludes AI pipeline tests; testpaths = code/tests)
python -m pytest -v

# By subdirectory
python -m pytest code/tests/engine -v                       # Engine unit tests
python -m pytest code/tests/behavioral -v                   # DebugRunner behavioral tests
python -m pytest code/tests/runners -v                      # Game runner tests
python -m pytest code/tests/rl -v                           # RL training tests

# By marker
python -m pytest -m scenario -v                             # YAML scenario tests only
python -m pytest code/tests/ai_pipeline -v                  # AI pipeline tests (opt-in)
python -m pytest -m "not slow" -v                           # Skip slow smoke tests

# RL training
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000

# Env smoke check
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"

# ONNX model export (requires PyTorch)
python code/tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx
python code/tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx

# Tauri desktop app (requires Rust toolchain; Python-free at runtime)
cd code/src-tauri && cargo tauri dev                     # development
cd code/src-tauri && cargo tauri build                   # production installers
cargo test --manifest-path code/src-tauri/Cargo.toml     # Tauri-layer unit tests

# Rust engine tests
cargo test --manifest-path code/digimon-engine/Cargo.toml
cargo test --manifest-path code/digimon-engine/Cargo.toml --test security_effects
cargo test --manifest-path code/digimon-engine/Cargo.toml --test test_cards_behavioral

# Engine debug CLI + MCP (see docs/DEBUG_MCP.md)
cargo build -p digimon-engine-cli -p digimon-engine-mcp
target/debug/digimon-engine-cli debug                      # interactive REPL
target/debug/digimon-engine-cli replay rec.json --step 47  # recording viewer
target/debug/digimon-engine-mcp --pool implemented         # MCP stdio server

# Training inspection MCP (see docs/TRAINING_MCP.md) — read-only cross-game
# inspection of RL training runs. Parallel to the engine MCP: engine MCP owns
# per-game forensics, this one owns runs/ + models/ filesystem artifacts
# (eval sidecars, TensorBoard metrics, recordings inventory, checkpoints).
pip install -r requirements-mcp.txt && pip install -e code/digimon-training-mcp
python -m digimon_training_mcp --runs-dir ./runs --models-dir ./models

# PyO3 bindings (build + install into active Python env)
cd code/digimon-engine-py && maturin develop

# Python-side Rust-backend parity test (uses Rust engine via env var)
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
```

## Working Rules

1. Keep tensor and action specs in sync with `game.py` and frontend constants.
2. Preserve headless engine behavior; UI reflects state, it does not own rules.
3. Do not bypass action masking in agent logic.
4. When updating phases/actions, update tests and both spec docs in the same change.
5. Keep docs stable: avoid stale hardcoded snapshot claims unless explicitly time-stamped.
6. When threading LSTM state during evaluation/inference, reset state to `None` at episode boundaries.
7. OpponentWrapper discards dense rewards from opponent steps; only terminal rewards pass through.
8. The desktop Tauri build must not link any Python runtime. All gameplay, inference, and deck tooling dispatch through Tauri `invoke()` into `digimon-engine`; trained models are downloaded at runtime via `code/src-tauri/src/models.rs` and cached under `dirs::data_dir()/digimon-tcg/models/`.
9. WebSocket state broadcasts must use `state_filter.py` — never send raw `to_ui_json()` to network clients.
10. ONNX policies must call `reset()` at episode boundaries for LSTM models (same rule as SB3 LSTM state threading).
11. Engine-only routers (`games.py`, `replays.py`, `simulations.py`, `deck_tools.py`) must not import from `digimon_gym.db.*` or `digimon_gym.ai.*`.
12. Training CLI modules (`pilot_training.py`, `gauntlet.py`, `deck_pool.py`) must not import from `digimon_gym.db.*`.
13. Desktop frontend builds use `VITE_BUILD_TARGET=desktop` to tree-shake admin/training UI.
14. `state_filter.py` must redact both `handIds` and `handCards` for opponents — never leak card metadata.
15. Game animation components (`DigivolveBanner`, `BattleEffect`) subscribe to `store.events` and track `lastSeqRef` to avoid replaying stale events.
16. `Game.surrender()` must emit the `surrender` event before calling `declare_winner()` so event listeners see the surrender before game_over.
17. The no-approximations policy applies identically to the Rust engine — no stubs, no auto-selections; every choice must surface through `pending_selection` so the RL action space sees it.
18. New Rust card effects are TDD: write a failing behavioral test under `code/digimon-engine/tests/` using `DebugRunner` (see `code/digimon-engine/tests/test_cards_behavioral.rs`) before implementing the `CardEffect` struct.
19. Before editing engine code in either language, check `docs/RUST_PYTHON_PARITY.md` for known divergences in the area — it's the authoritative cross-engine state.
20. `code/digimon-engine-py/src/lib.rs` must preserve the Python player-ID convention (Python 1/2 ↔ Rust 0/1 translation at the binding boundary); callers on both sides depend on it.
21. Do not author new Python-side card scripts for cards already implemented in Rust — cards migrate one direction only (Python → Rust) and are then owned by Rust.
22. `code/engine_py_legacy/` is sunset reference material. Production code (`code/server/`, `code/digimon_gym/`, `code/digimon-engine/`, `code/digimon-engine-py/`) must not import from `engine_py_legacy.*`. Tests in `code/engine_py_legacy/tests/` are excluded from default pytest collection.
23. Trained model artifacts go to `<repo_root>/models/<run_id>/`. Training entrypoints (`pilot_training`, `architect_training`) default to this location. The hosted API's `/models/manifest.json` scans this directory.
24. All source lives under `code/`. The repo root holds docs, infra (`Dockerfile*`, CI workflows), agent config (`.claude/`), runtime data (`data/`, `qa/`, `DCGO/`), and project-level configs (`Cargo.toml`, `pyproject.toml`, `requirements*.txt`). Do not add new top-level source dirs — extend `code/` instead.
25. **OnDeletion handlers fire post-trash (2026-05-23).** Permanent deletion runs through the batched flow at `Game::delete_permanents_batch` — `OnDeletion` handlers execute AFTER the carrier has moved to trash. Read pre-removal state via `ctx.deleted_self_dp()` / `_level()` / `_cost()` / `_names()` / `_traits()` / `_source_count()` / `_digisources()` (or `ctx.deleted_object_snapshot()`), NOT via live `ctx.game.player(handle.player).battle_area.get(handle.index)`. New keywords needing post-trash work must do it inline in the OnDeletion handler — do not reintroduce side-channel slots like the retired `pending_post_deletion_replays`. See `docs/RUST_ENGINE_API.md` §"Deletion lifecycle — batched flow" for the contract.
26. **BO3 match training is the default Gym episode shape (2026-05-24).** `--match-format bo3` (default) makes one Gym episode equal one best-of-three match: same deck pair across all 3 games, LSTM hidden state carries across games within the match, total step counter accumulates. `--match-format single` retains the legacy one-game episode. The `MatchEnv` wrapper sits between `OpponentWrapper` and any deck-pool wrappers — so deck-pool sampling fires once per match, not per game. Concede (action `93`) is legal at every agent decision point and routes to `Game::concede(player)` regardless of `pending_selection` state. `SelectPlayOrder` (actions `94` / `95`) is engine-driven via `Game::request_play_order_selection(loser_pid)` between games; the loser of the previous game picks first / second for the next. `MatchEnv` reads the result via `runner.take_play_order_choice()` and uses the seed-parity trick (`Game::new` uses `seed % 2` for 2-player first-player selection) to align the next game's first player. The reward calibration is in `openspec/changes/add-bo3-match-training/design.md` §D9; do not modify per-game or per-match magnitudes without also updating the spec.

## Documentation

Detailed reference docs live in `docs/` — see [docs/INDEX.md](docs/INDEX.md) for the full list.

Key references:
- **Architecture**: `docs/ARCHITECTURE.md` — API surface, RL contracts, frontend components, desktop distribution
- **Spec contracts**: `docs/TENSOR_SPEC.md` (obs tensor), `docs/ACTION_SPEC.md` (action space)
- **Tools**: `docs/TOOLS.md` — card pipeline, transpiler, Pinecone, model export, new-set workflow
- **Training**: `docs/TRAINING_RUNBOOK.md` + `AGENTS.md` (wrapper chain, gauntlet, pipeline)
- **Rules**: `docs/RULES_CONTEXT.md` — official Digimon TCG rules reference
- **Hosted-API deployment**: `docs/DEPLOYMENT.md` — DigitalOcean topology, env vars, bootstrap
- **Model catalog**: `docs/MODEL_CATALOG.md` — ONNX upload/download pipeline, storage backends, desktop cache

**Rust engine (target source of truth — read all three before editing engine code):**
- **Design / phase plan**: `docs/superpowers/specs/2026-04-15-rust-engine-rewrite-design.md` — north-star architecture, crate structure, phase roadmap
- **Scripting API**: `docs/RUST_ENGINE_API.md` — `EffectContext` API, `Effect` builder, `CardEffect` trait, TDD walkthrough for new cards
- **Cross-engine parity**: `docs/RUST_PYTHON_PARITY.md` — live divergence tracker and per-phase progress; transitional, retired when the Python engine is

**Rust card scripting**: a Rust-focused `batch-fix-cards` skill (analogous to the existing Python `/batch-fix-cards` but targeting `code/digimon-engine/src/cards/` + `code/digimon-engine/tests/`) is planned for the forthcoming card-migration phase. Until then, author new Rust card effects directly per the TDD walkthrough in `RUST_ENGINE_API.md`.

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
