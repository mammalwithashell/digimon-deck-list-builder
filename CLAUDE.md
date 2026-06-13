# CLAUDE.md - AI Assistant Guide

## Scope

This document is a current-state engineering guide for working in this repository.
It focuses on stable contracts and implementation shape, not static snapshot metrics.

## Project Vision

This is a **Digimon TCG simulator** built for both human play and **RL agent training/deckbuilding**. The engine faithfully implements card effects so RL agents can learn optimal play strategies across the full card pool.

- **No-Approximations Policy**: every card effect must faithfully implement all card text; no stubs, no auto-selections; every choice must be exposed to the RL action space so agents can learn to make optimal decisions; gaps are marked BLOCKED and logged to `qa/archetype-qa/engine-gaps.md`

### Source priority for card / keyword / rules questions

When questioning *what a card or keyword should do* — for instance, "is Fragment optional?", "what does Save target?", "does Progress block X?" — consult sources in this order. **DCGO is battle-tested and community-curated; trust it over `RULES_CONTEXT.md` (which is LLM-generated) and over the card-text JSON (which is API-ingested and not always accurate).**

1. **Official Rules Manual PDF** — `Digimon TCG resources/general_rule.pdf` is the **canonical source of truth** for rules, keyword semantics, and timing (keyword semantics live in §16; rule numbers like `16-36` cite it directly). `glossary.pdf` defines keywords. **This is NOT LLM-generated — reference it directly and often.** It is currently under-used; read it (via the Read tool's `pages` arg) when implementing or reasoning about any keyword/timing rule rather than relying on the decomposed text.
2. **DCGO C# source** (`<base-repo>/DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`) — the community-curated, battle-tested behavioral implementation. It is the authority for **how a specific card actually resolves** (processing order, interaction edges, multi-pick UI flow) and **outranks the card-text JSON for what a card does**. **DCGO lives in the base repo, not per-worktree (see rule 29). From a worktree, the local `./DCGO` is an intentionally-empty placeholder — do NOT `git submodule update --init` it (that clones a multi-GB Unity checkout per worktree and bloats disk). Resolve the populated copy in the base repo: `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"` and read from there. Do NOT skip DCGO and fall back to lower-trust sources because the worktree copy looks empty.** (C# filenames use underscores: `BT17-001` → `BT17_001.cs`.)
3. **`data/card_overrides.json`** then **`data/cards.json`** — the printed card text (`effect_text`, `inherited_text`, `security_text`). `cards.json` is ingested from the digimoncard.io API and is **not always accurate**; `card_overrides.json` holds our hand-maintained corrections and is trusted over raw `cards.json` (overrides survive re-ingestion). Use the text as the no-approximations *target*, but when it conflicts with DCGO's behavior, DCGO wins for now.
4. **Fandom wiki** — `https://digimoncardgame.fandom.com/wiki/<CARD-ID>` for card-specific text + community-curated ruling notes (quirky interactions, errata). Useful when the printed text is terse.
5. **`docs/RULES_CONTEXT.md`** — an **LLM-generated decomposition** of `general_rule.pdf`. Treat it as a convenience index / starting pointer only; it has been wrong. Anything it asserts is overridden by the official PDF (#1) and DCGO (#2) — verify against those before relying on it.

When DCGO and the official PDF disagree on a rules question, the PDF governs; when DCGO and the card-text JSON disagree on what a card does, DCGO governs. The retired guidance ("do not skip ahead to DCGO", "RULES_CONTEXT.md is canonical") is intentionally reversed here.

**Look at the actual card.** For *printed text* ("what does it say") the highest-fidelity source is the **card image itself** — it is the literal card face, so it outranks the API-ingested `cards.json` (and ties with `card_overrides.json`, which only patches sparse fields). A complete local image mirror exists and the `Read` tool renders `.webp` natively. When reasoning about, implementing, fixing, or QA'ing a card — or a whole archetype — use the **`/digimon-card-lookup`** skill (`.claude/skills/digimon-card-lookup/`): it resolves a card ID, a card *name* (all printings — e.g. "Wormmon" is 8 distinct cards), or an *archetype* to image paths + text via `resolve_cards.py`, then you `Read` the images. This governs *printed text only*; DCGO C# / `general_rule.pdf` remain authoritative for *behavior*. A `UserPromptSubmit` hook (`.claude/hooks/digimon_card_image_hint.py`) reminds you with image paths whenever a prompt mentions card IDs.

### Rust pivot (in progress)

The project is migrating to a **Rust engine as the source of truth** (`code/digimon-engine/`). Python is retained only for the FastAPI server (P2P games, lobby, auth) and RL training (gym/SB3); both call into the Rust engine via PyO3 bindings (`code/digimon-engine-py/`). Card scripting is **DSL-first**: new cards are authored as YAML specs in `code/digimon-engine/cards/<set>/`, lowered to `Effect`/`CardEffect` by the `digimon-dsl` crate, and TDD-driven by DebugRunner tests in `code/digimon-engine/tests/cards_behavioral/<set>/`. When a card can't be expressed, **widen the substrate rather than routing around it** — add DSL vocabulary or an engine primitive — so each hard card makes the next one cheaper and per-card effort trends down over time. Hand-written Rust effects (`code/digimon-engine/src/cards/raw_rust/`) are a last-resort escape hatch (see rule 28). The no-approximations policy applies identically in Rust. `docs/RUST_PYTHON_PARITY.md` is a **transitional** tracker of cross-engine divergences — it exists only until the Python engine is retired.

## Tech Stack

- **Engine (target)**: Rust — `code/digimon-engine/` library crate, `code/digimon-engine-py/` PyO3 bindings
- **Engine (sunset)**: Python 3.11+, `code/engine_py_legacy/engine/` — reference material only; not importable from production code
- **Backend**: FastAPI + Uvicorn, SQLAlchemy + PostgreSQL (hosted API only); binds to Rust engine via PyO3
- **Frontend**: React 18 + TypeScript + Vite, Zustand state management
- **Desktop**: Tauri v2 (Rust shell) — Python-free; gameplay + inference + deck tools run entirely in the embedded `digimon-engine` crate, and AI models are fetched at runtime from the hosted API's manifest
- **RL**: Gymnasium, Stable-Baselines3, ONNX Runtime for inference; env drives the Rust engine via PyO3
- **AI Pipeline**: Claude API, Pinecone vector DB, git worktrees
- **C# Reference**: DCGO submodule (`DCGO/`) — community-curated, battle-tested behavioral implementation. The high-trust reference for how a specific card resolves; it **outranks the card-text JSON and `RULES_CONTEXT.md`** (only the official `general_rule.pdf` ranks higher for pure rules questions). See "Source priority" above. DCGO lives in the **base repo**, not per-worktree — from a worktree read it at `$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO` rather than initializing the local placeholder (rule 29).

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
    │   │   ├── cards/                  # Card-effect Rust modules:
    │   │   │   ├── raw_rust/           #   hand-written CardEffect escape hatch (rule 28)
    │   │   │   ├── keyword_effects.rs  #   shared keyword machinery
    │   │   │   ├── tokens/             #   token registry
    │   │   │   └── test/               #   TEST-001..022 hand-written worked examples
    │   │   ├── runners/               # HeadlessRunner (RL-shaped API)
    │   │   └── debug_runner.rs        # Deterministic test harness
    │   ├── cards/<set>/               # YAML DSL card specs (primary path) — ad1, bt1..bt18, ...
    │   └── tests/                     # Integration tests (engine_core, tensor_and_mask, etc.)
    │       └── cards_behavioral/<set>/  # Per-card DebugRunner behavioral tests (TDD)
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
    │   ├── dsl-lint/              # DSL linter (Cargo workspace member)
    │   ├── action-space-export/   # 2192-action descriptor → DCGO ActionSpace.cs codegen (rule 27)
    │   ├── dcgo-replay/           # Replay DCGO JSONL recordings through the engine (parity oracle)
    │   └── dcgo-bc-emitter/       # DCGO recordings → (obs, mask, action) numpy shards for BC
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

**Write-capable dev/test MCP** (local + dev only — documented exception to the read-only rule above):
- `digimon-scenario-mcp` (Python package, `code/digimon-scenario-mcp/`) — stages, snapshots, and authors game-state scenario tests. It **mutates game state and writes files** (`qa/scenarios/`, `code/frontend/e2e/`) — that is its purpose. Bounded: dev/test-only, never bundled into any production build, never imported by `server.*` / `digimon_gym.*`, and talks only to dev-gated surfaces (the hosted-API `/debug` router in browser-dev mode, and the feature-gated Tauri debug bridge on desktop via `target: browser|desktop`). See `docs/SCENARIO_MCP.md` and the `add-scenario-capture-mcp` change.

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
python -m digimon_gym.agents.pilot_training --opponent pool --opponent-pool-manifest pool.json --timesteps 1000000  # fictitious self-play vs frozen champions (--self-play is RETIRED)
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000
python -m digimon_gym.agents.pilot_training --archetypes rocks,ts-olympos --timesteps 500000  # scope the deck pool

# Cloud training (see docs/CLOUD_TRAINING.md) — image is built/pushed by CI on tag push
docker compose -f ops/training/docker-compose.watch.yml up -d   # TensorBoard sidecar over ./runs
scripts/sync_cloud_runs.sh                                       # mirror remote runs/ back locally for the MCP

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
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral   # per-set YAML DSL card tests

# DSL tooling (schema + lint for YAML card specs under code/digimon-engine/cards/)
cargo run -p dsl-schema-export                             # regenerate the DSL JSON schema
cargo run -p dsl-lint -- code/digimon-engine/cards         # lint all YAML card specs

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
18. New Rust cards are TDD and **DSL-first** (see rule 28): write a failing behavioral test under `code/digimon-engine/tests/cards_behavioral/<set>/` using `DebugRunner` before authoring the YAML spec in `code/digimon-engine/cards/<set>/`. Only fall back to a hand-written `CardEffect` in `src/cards/raw_rust/` when the DSL genuinely can't express the card.
19. Before editing engine code in either language, check `docs/RUST_PYTHON_PARITY.md` for known divergences in the area — it's the authoritative cross-engine state.
20. `code/digimon-engine-py/src/lib.rs` must preserve the Python player-ID convention (Python 1/2 ↔ Rust 0/1 translation at the binding boundary); callers on both sides depend on it.
21. Do not author new Python-side card scripts for cards already implemented in Rust — cards migrate one direction only (Python → Rust) and are then owned by Rust.
22. `code/engine_py_legacy/` is sunset reference material. Production code (`code/server/`, `code/digimon_gym/`, `code/digimon-engine/`, `code/digimon-engine-py/`) must not import from `engine_py_legacy.*`. Tests in `code/engine_py_legacy/tests/` are excluded from default pytest collection.
23. Trained model artifacts go to `<repo_root>/models/<run_id>/`. Training entrypoints (`pilot_training`, `architect_training`) default to this location. The hosted API's `/models/manifest.json` scans this directory.
24. All source lives under `code/`. The repo root holds docs, infra (`Dockerfile*`, CI workflows), agent config (`.claude/`), runtime data (`data/`, `qa/`, `DCGO/`), and project-level configs (`Cargo.toml`, `pyproject.toml`, `requirements*.txt`). Do not add new top-level source dirs — extend `code/` instead.
25. **OnDeletion handlers fire post-trash (2026-05-23).** Permanent deletion runs through the batched flow at `Game::delete_permanents_batch` — `OnDeletion` handlers execute AFTER the carrier has moved to trash. Read pre-removal state via `ctx.deleted_self_dp()` / `_level()` / `_cost()` / `_names()` / `_traits()` / `_source_count()` / `_digisources()` (or `ctx.deleted_object_snapshot()`), NOT via live `ctx.game.player(handle.player).battle_area.get(handle.index)`. New keywords needing post-trash work must do it inline in the OnDeletion handler — do not reintroduce side-channel slots like the retired `pending_post_deletion_replays`. See `docs/RUST_ENGINE_API.md` §"Deletion lifecycle — batched flow" for the contract.
26. **BO3 match training is the default Gym episode shape (2026-05-24).** `--match-format bo3` (default) makes one Gym episode equal one best-of-three match: same deck pair across all 3 games, LSTM hidden state carries across games within the match, total step counter accumulates. `--match-format single` retains the legacy one-game episode. The `MatchEnv` wrapper sits between `OpponentWrapper` and any deck-pool wrappers — so deck-pool sampling fires once per match, not per game. Concede (action `93`) is legal at every agent decision point and routes to `Game::concede(player)` regardless of `pending_selection` state. `SelectPlayOrder` (actions `94` / `95`) is engine-driven via `Game::request_play_order_selection(loser_pid)` between games; the loser of the previous game picks first / second for the next. `MatchEnv` reads the result via `runner.take_play_order_choice()` and uses the seed-parity trick (`Game::new` uses `seed % 2` for 2-player first-player selection) to align the next game's first player. The reward calibration is in `openspec/changes/add-bo3-match-training/design.md` §D9; do not modify per-game or per-match magnitudes without also updating the spec.
27. **DCGO recorder maintenance (2026-05-26).** The DCGO mod intercepts gameplay at five chokepoints across two files: in `DCGO/Assets/Scripts/Script/TurnStateMachine.cs` — `QueueMainPhaseAction` (every main-phase decision), `SetRedraw` (mulligan), `StartGame` (game lifecycle start / deck capture), `EndGame` (game lifecycle end). In `DCGO/Assets/Scripts/Script/UserSelectionManager.cs` — `SetIntForPlayer` and `SetBoolForPlayer` (every selection response). Plus three reveal chokepoints in `DCGO/Assets/Scripts/Script/CardController.cs` for PvP: `DrawClass.Draw()`, `IBreakSecurity.SecurityCheck()`, `IAddTrashCardsFromLibraryTop.AddTrashCardsFromLibraryTop()`. When rebasing the DCGO submodule onto a newer upstream commit, verify every hook is still in place at its chokepoint — if upstream refactored the chokepoint, the hook needs to move. After ANY change to `code/digimon-engine/src/action/space.rs` (constants, ranges, encoder formulas), regenerate `ActionSpace.cs` in the **base-repo** DCGO (rule 29) via `cargo run -p action-space-export | python code/tools/action-space-export/emit_csharp.py --out "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO/Assets/Scripts/Script/Recording/ActionSpace.cs"` and commit the diff (in the base repo, where the submodule is checked out). CI (`.github/workflows/action-space-codegen-drift.yml`) catches drift, but local development should regen explicitly when touching action-space code. The JSONL recording schema is described in `docs/DCGO_RECORDING_SCHEMA.md`; the build setup in `docs/DCGO_BUILD.md`.
28. **Card scripting is DSL-first (2026-05-29).** The default path for a new card is a YAML spec in `code/digimon-engine/cards/<set>/`, lowered to `Effect`/`CardEffect` by the `digimon-dsl` crate. The intent is a compounding-coverage flywheel: when a card needs something the DSL lacks, **widen the substrate instead of routing around it** — add DSL vocabulary (lower it in `code/digimon-dsl/`, log the gap to `qa/dsl-vocab-gaps.md`) or, if the engine itself lacks the primitive, add it and log to `docs/RUST_ENGINE_GAPS.md`. That investment amortizes across every later card, so implementation complexity should decline as coverage grows. Hand-writing a `CardEffect` in `code/digimon-engine/src/cards/raw_rust/` is a **last resort** for cards the DSL fundamentally can't express — reaching for it because it's faster in the moment starves the vocabulary and breaks the flywheel. Verdicts: DSL cards tracked in `qa/qa-reports/validated_cards_dsl.json`; hand-written in `qa/qa-reports/validated_cards.json`. Skills: `/batch-implement-cards-rust-dsl` and `/implement-rust-dsl-archetype` (YAML path), `/assess-archetype-rust` (pre-flight gap audit → `docs/RUST_ENGINE_GAPS.md` + a fix-plan under `.claude/plans/`), `/batch-implement-cards-rust` (hand-written escape-hatch path).
29. **DCGO lives in the base repo; worktrees reference it, never clone it (2026-05-29).** The `DCGO/` submodule is a multi-GB Unity checkout. It is initialized **once in the base repo** (the main worktree) and kept there. In any linked worktree, `./DCGO` is an intentionally-empty placeholder — **do NOT run `git submodule update --init DCGO` in a worktree** (it clones a full per-worktree copy and is the main driver of disk bloat / forced worktree pruning). Resolve the base copy instead: `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`. Use `$BASE_DCGO` for **both** reading card C# (source-priority #2) **and** writing — including DCGO mod edits and the rule-27 `ActionSpace.cs` codegen — because Unity is pointed at the base copy, so all mod work belongs there. If `$BASE_DCGO` itself is somehow missing, initialize it in the base repo (`cd "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")" && git submodule update --init DCGO`), not in the worktree. The worktree's empty `DCGO/` placeholder is the clean state — leave it; never `git submodule deinit` from a worktree (it edits the shared `.git/config` and can disturb the base submodule).
30. **In-run win rate is NOT a cross-mode learning signal; rank models on anchored evaluation (2026-05-31).** The training eval reuses the *training* opponent (`eval_env_fn = make_env(opponent=opponent, ...)`), so the reported win rate means a different thing in every mode and is **degenerate under self-play** — a model-vs-mirror that pins near 50% (or, if the opponent seat goes passive, reads a meaningless 100%). It is also not comparable across runs/modes. **Never claim a model improved from the in-run / mirror / self-play eval.** Judge a model with **anchored evaluation** instead: play it against *fixed* references — greedy (skill floor) plus frozen champions from the registry — **seat-balanced** (alternate first-player via `seed % 2`), on one comparable scale. Tooling: `code/digimon_gym/agents/anchored_eval.py` + the champion registry (`code/digimon_gym/agents/champion_registry.py`, `models/champions/registry.json`) + the CLI `code/tools/anchored_eval_cli.py` (use `--deck-pool-snapshot <run>/deck_pool_snapshot.json` for the exact training decks, and an adequate `--n` — small n is deck-luck noisy). The opponent MUST be driven via `OpponentWrapper`; a raw `DigimonEnv` leaves player 2 passive (first player always wins → fake ~50%/100%). For robustness, estimate exploitability with a forward-only PPO exploiter. See `docs/MODEL_EVALUATION.md` and the `add-model-evaluation-harness` change. **Update (2026-06-11, `harden-training-pipeline`):** `opponent="self-play"` is **retired** and fails at startup (P1-perspective observations made the mode structurally unsound — use `opponent="pool"` with a `champion_admin.py emit-pool` manifest); training runs now also log an **in-training anchored panel** (`anchored_eval_freq`, default every 100k steps) as `pilot/anchored/*` scalars + `anchored_evals.jsonl`, so a collapsing run is visible mid-run — but promotion decisions still come only from the post-hoc anchored frame (runbook §14 "Standing cadence").

31. **Rust build isolation — per-worktree `CARGO_TARGET_DIR` (2026-06-13).** Rust artifacts are isolated per git worktree under `D:\cargo-target\<worktree-name>` (D: has ~1.7 TB free; C: was the original disk squeeze). A **single shared** `CARGO_TARGET_DIR` across worktrees causes cargo to link one worktree's crate `.rmeta` into another's build → **phantom compile errors in files you never edited** (classic tell: `non-exhaustive patterns: …OnAnyLink not covered` in `dsl_cards/timing_map.rs` while your own `cargo check --lib` passes — that variant only exists in another worktree's in-flight edit). The isolation is wired via User env vars: `BASH_ENV=C:/Users/james/.bashrc` (makes the harness's non-interactive bash source `~/.bashrc`, which derives the per-worktree dir from cwd), `CARGO_TARGET_BASE=D:\cargo-target`, no global `CARGO_TARGET_DIR`, plus shared sccache (`RUSTC_WRAPPER`, `SCCACHE_DIR=D:\sccache`, `SCCACHE_CACHE_SIZE=40G`). **These activate only after the Claude app restarts** (the running harness captured its env at launch); within a pre-restart session the old shared `D:\cargo-target` is still inherited, so prefix cargo commands with an explicit `CARGO_TARGET_DIR='D:\cargo-target-wt\<name>'` to build/verify in isolation. If you hit a compile error in code you didn't touch, suspect contamination before debugging your own change. See memory `reference-cargo-target-per-worktree`.

## Documentation

Detailed reference docs live in `docs/` — see [docs/INDEX.md](docs/INDEX.md) for the full list.

Key references:
- **Architecture**: `docs/ARCHITECTURE.md` — API surface, RL contracts, frontend components, desktop distribution
- **Spec contracts**: `docs/TENSOR_SPEC.md` (obs tensor), `docs/ACTION_SPEC.md` (action space)
- **Tools**: `docs/TOOLS.md` — card pipeline, transpiler, Pinecone, model export, new-set workflow
- **Training**: `docs/TRAINING_RUNBOOK.md` + `AGENTS.md` (wrapper chain, gauntlet, pipeline)
- **Model evaluation**: `docs/MODEL_EVALUATION.md` — why the in-run win rate lies (degenerate under self-play), the anchored reference frame (greedy + frozen champions, seat-balanced), the layered eval stack, the Elo ladder + champion registry + exploiter tools, gated self-play, and the equilibrium-methods horizon (depends on `make-engine-cloneable`). See rule 30.
- **Cloud training**: `docs/CLOUD_TRAINING.md` — Path A (RunPod GPU, LSTM/self-play) vs Path B (Hetzner/DO CPU, MLP-vs-greedy); published `Dockerfile.training` image, the `ops/training/docker-compose.watch.yml` TensorBoard sidecar, and the `scripts/sync_cloud_runs.sh` run-mirror; local VRAM mitigations
- **Rules**: `Digimon TCG resources/general_rule.pdf` — **canonical** rules source of truth (+ `glossary.pdf` for keywords). `docs/RULES_CONTEXT.md` is an LLM-generated decomposition — convenience index only, lower trust than the PDF and DCGO (see "Source priority")
- **Hosted-API deployment**: `docs/DEPLOYMENT.md` — DigitalOcean topology, env vars, bootstrap
- **Model catalog**: `docs/MODEL_CATALOG.md` — ONNX upload/download pipeline, storage backends, desktop cache
- **DCGO recording pipeline**: `docs/DCGO_BUILD.md` (build the mod) + `docs/DCGO_RECORDING_SCHEMA.md` (JSONL format) — modded DCGO client that records games as 2192-action-space JSONL, consumed by `code/tools/dcgo-replay/` as an additional Rust-engine faithfulness oracle
- **Interactive replay bug-hunting**: `docs/DEBUG_MCP.md` + the `/replay-bug-hunt` skill — step a single recorded game (native eval/self-play OR DCGO PvP/bot) through the engine via the `digimon-engine-mcp` stepping + scanner tools to find/localize/confirm engine bugs. Mode 1 (DCGO source) is differential against the DCGO oracle; Mode 2 (native source) judges faithfulness vs card text + `general_rule.pdf` + DCGO C#. The skill is the **microscope**; `dcgo-replay` is the **funnel** — both share one replay core (`ReplaySession`/`LiveGame`). Back-stepping is reset-and-replay, not snapshots (contract in `docs/RUST_ENGINE_API.md` §"Reset-and-replay contract")
- **Archetype interaction testing (capstone)**: the `/archetype-interaction-test-author` skill — the **proactive** third bug-discovery mode (alongside the two replay modes). It researches an archetype as a *system*, emits a durable model (`qa/archetype-qa/<archetype>-model.md`), then authors multi-card **interaction tests** in `code/digimon-engine/tests/archetypes/<slug>.rs` (home for combos that span sets/cards; fixtures in `tests/archetypes/support.rs`) plus four **static archetype tests** — deck-legality, coverage gate, smoke games, combo-presence — via the `archetype-static-tests` crate (`cargo run -p archetype-static-tests -- "<archetype>"`, verdicts in `qa/qa-reports/archetype_interactions.json`). It runs **after** cards are implemented and per-card tests are green (composes with `/assess-archetype-rust` + `/batch-implement-cards-rust-dsl`), never re-implements cards, and routes confirmed failures to the shared gap trackers. Pattern reference: `docs/RUST_DSL_TEST_API.md` §2

**Rust engine (target source of truth — read all three before editing engine code):**
- **Design / phase plan**: `docs/superpowers/specs/2026-04-15-rust-engine-rewrite-design.md` — north-star architecture, crate structure, phase roadmap
- **Scripting API**: `docs/RUST_ENGINE_API.md` — `EffectContext` API, `Effect` builder, `CardEffect` trait, TDD walkthrough for new cards
- **Cross-engine parity**: `docs/RUST_PYTHON_PARITY.md` — live divergence tracker and per-phase progress; transitional, retired when the Python engine is

**Rust card scripting (DSL-first — see rule 28)**: cards are authored as YAML specs in `code/digimon-engine/cards/<set>/`, lowered by the `digimon-dsl` crate, with per-card DebugRunner tests in `code/digimon-engine/tests/cards_behavioral/<set>/`. Drive the work with `/batch-implement-cards-rust-dsl` or `/implement-rust-dsl-archetype`; run `/assess-archetype-rust` first to audit which engine/DSL primitives an archetype needs. The DSL schema is generated by `code/tools/dsl-schema-export/` and linted by `code/tools/dsl-lint/`. For the rare card the DSL can't express, hand-write a `CardEffect` in `src/cards/raw_rust/` per the TDD walkthrough in `RUST_ENGINE_API.md` (driven by `/batch-implement-cards-rust`). To author an **entire release set** (a booster like BT17/EX12 rather than a single archetype), use `/author-set <SET>`: it refreshes the set from digimoncard.io, runs the DCGO-oracle keyword gate (auto-ingest keywords DCGO implements, flag-for-human those it doesn't) *before* implementation, clusters the ~100 cards into archetype slices, then fans out the per-slice authoring/combo-test skills via the `author-set` Workflow. Tooling: `code/tools/author_set/` (see its README); design: `openspec/changes/add-author-set-workflow/`.

## QA Artifacts

- `qa/archetype-qa/` — per-archetype implementation QA, engine API reference, engine gaps tracker; `qa/archetype-qa/dsl/` holds per-archetype DSL/engine gap inputs
- `qa/qa-reports/` — dated gameplay test reports + card verdict trackers: `validated_cards_dsl.json` (YAML DSL cards) and `validated_cards.json` (hand-written / Python)
- **Gap trackers** (rule 28 — widen the substrate, don't route around): `qa/dsl-vocab-gaps.md` (missing DSL vocabulary) and `docs/RUST_ENGINE_GAPS.md` (missing engine primitives). Per-archetype fix plans land under `.claude/plans/rust-engine-gaps-*.md`.

## Pinecone MCP Integration

The `/implement-archetype` skill uses Pinecone (`digimon-engine` index) for sub-agent retrieval. MCP server: `@pinecone-database/mcp` in `.mcp.json`. Requires `PINECONE_API_KEY` env var.

| Namespace | Content | ~Vectors |
|-----------|---------|----------|
| `engine-api` | Engine API reference + decomposed engine source | ~300 |
| `card-scripts` | Python scripts (frozen + generated) + C# reference | ~6,000 |
| `card-metadata` | Per-card entries from cards.json | ~4,000 |
| `rules-docs` | RULES_CONTEXT.md, ACTION_SPEC.md, TENSOR_SPEC.md, engine-gaps.md | ~100 |

See `docs/TOOLS.md` §5 for ingestion and verification commands.
