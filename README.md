# Digimon TCG Simulator

A faithful Digimon Card Game rules engine written in Rust, with a Gymnasium RL environment, a React frontend, a FastAPI hosted server, and a Tauri v2 desktop app. It is built for two audiences: people who want to play, and RL agents that learn to play and build decks across the full card pool.

**Status:** pre-alpha, under active development. The Rust engine is the single source of truth. The legacy Python engine was removed in June 2026, so every surface (desktop, hosted PvP, RL training, tools) now runs on Rust.

## What This Project Does

- **Rust rules engine.** `code/digimon-engine/` implements the comprehensive rules: turn flow, memory, digivolution, combat, security, effect timing, and every §16 keyword. Python reaches it through PyO3 bindings (`code/digimon-engine-py/`, imported as `digimon_engine`, class `RustHeadlessGame`).
- **No-approximations policy.** Every card effect implements all of its printed text: no stubs and no auto-selections. Every choice surfaces through `pending_selection`, so it shows up in the RL action space and agents can learn to make it. Anything the engine can't yet express is logged to a gap tracker instead of approximated.
- **DSL-first card scripting.** Cards are YAML specs in `code/digimon-engine/cards/<set>/`, lowered to engine effects by the `digimon-dsl` crate and test-driven with `DebugRunner` behavioral tests. When a card needs vocabulary the DSL lacks, the DSL or engine is widened instead of working around the gap, so each hard card makes the next one cheaper.
- **Verified against an oracle.** The community DCGO C# client (`DCGO/` submodule, modded to record games) is the behavioral reference. Recorded DCGO games are replayed through the engine (`dcgo-replay`), and scripted per-clause exams make DCGO and the engine answer "what does this card do *here*" side by side.
- **Gymnasium RL environment.** `DigimonEnv` uses versioned observation-tensor profiles, a fixed 2192-action discrete space with phase-aware masking, composable reward profiles, and best-of-three match episodes by default.
- **Training and evaluation stack.** MaskablePPO and recurrent (LSTM) pilots, MetaGauntlet and pool/league opponents, Architect agents for deck optimization, anchored evaluation against fixed references (greedy plus frozen champions), an Elo ladder, and exploiter-based robustness checks.
- **Search and self-play.** Determinized PUCT-MCTS over worlds sampled from the acting player's information set, with an AlphaZero-style self-play generation loop. This is built on `Game: Clone`.
- **Multiplayer PvP.** WebSocket real-time play with lobby matchmaking, rooms and join codes, and spectating. Broadcasts go through hidden-information filtering.
- **Desktop app.** A Tauri v2 shell embeds the Rust engine directly, with no Python at runtime. It runs local games against AI agents, deck tools, and ONNX inference. Trained models download from the hosted API's manifest, and releases ship through a signed auto-updater.

## Architecture

```
   ┌───────────────────────────────────────────────────────────┐
   │              React Frontend (TypeScript, Vite)             │
   │  GamePage · DeckBuilder · Lobby/Rooms · Models · Admin*    │
   │  (*admin/training UI tree-shaken from the desktop build)   │
   └─────────┬──────────────────────────────┬──────────────────┘
   invoke()  │ desktop                      │ HTTPS / WebSocket (web)
   ┌─────────▼──────────────────┐   ┌───────▼────────────────────────┐
   │  Tauri v2 Desktop Shell     │   │  Hosted API (FastAPI, Python)  │
   │  (Rust, no Python runtime)  │   │  PvP · lobby · auth · decks ·   │
   │  links digimon-engine;      │   │  recordings · admin AI ·        │
   │  ONNX inference; model      │   │  /models/manifest.json          │
   │  cache + auto-updater       │   │  state_filter.py redaction      │
   └─────────┬──────────────────┘   └───────┬────────────────────────┘
             │ in-process                   │ PyO3 (digimon_engine)
             │                              │
   ┌─────────▼──────────────────────────────▼──────────────────────┐
   │                 digimon-engine (Rust) — source of truth        │
   │  game state machine · combat · effect queue · selections ·     │
   │  observation tensors · 2192-action mask · inference · search   │
   │        ▲ YAML card specs lowered by digimon-dsl                │
   └────────┬───────────────────────────────────────────────────────┘
            │ PyO3 (digimon_engine)
   ┌────────▼────────────────────┐      ┌──────────────────────────┐
   │  DigimonEnv (Gymnasium)     │      │  DCGO/ (C# submodule)     │
   │  pilot_training · gauntlet  │      │  behavioral oracle:       │
   │  league · anchored eval ·   │      │  recorded games →         │
   │  AlphaZero trainer → ONNX   │      │  dcgo-replay, clause exams│
   └─────────────────────────────┘      └──────────────────────────┘
```

The project ships as three deployable services:

1. **Desktop app** (`code/src-tauri/`): local games against AI, plus deck tools. It is Python-free.
2. **Hosted API** (`code/server/api.py`): PvP, lobby, auth, user data, recordings, admin AI, and the model manifest.
3. **Training CLI** (`python -m digimon_gym.agents.pilot_training`): standalone RL training with no HTTP server and no database.

## Repository Map

All source lives under `code/`. The repo root holds docs, infra, agent config, runtime data, and project-level configs.

| Path | Purpose |
|---|---|
| `code/digimon-engine/` | Rust game engine (source of truth) |
| `code/digimon-engine/src/game/`, `game_actions/` | Turn state machine and tier-2 game-action facades (play, digivolve, sources, zones) |
| `code/digimon-engine/src/effect.rs`, `effect_context/`, `effect_queue.rs` | Card-scripting API and the triggered-effect queue |
| `code/digimon-engine/src/combat/`, `selection.rs`, `resume.rs` | Attack state machine, pending selections, resumable selection VM |
| `code/digimon-engine/src/observation/`, `tensor_profiles/`, `action/` | Versioned observation tensors and the 2192-action space, mask, and decoder |
| `code/digimon-engine/src/inference/`, `search/`, `selfplay/` | ONNX inference (MLP + LSTM), determinized MCTS, self-play |
| `code/digimon-engine/src/dsl_cards/`, `cards/raw_rust/` | DSL card runtime, plus the hand-written `CardEffect` escape hatch |
| `code/digimon-engine/cards/<set>/` | YAML card specs (primary card-authoring path) |
| `code/digimon-engine/tests/cards_behavioral/<set>/` | Per-card `DebugRunner` behavioral tests |
| `code/digimon-engine/tests/archetypes/` | Multi-card archetype interaction (combo) tests |
| `code/digimon-dsl/` | Card-scripting DSL crate: parsing, validation, lowering |
| `code/digimon-engine-py/` | PyO3 bindings (`RustHeadlessGame`), built with `maturin` |
| `code/digimon-engine-cli/` | Debug REPL, recording replay viewer, self-play driver |
| `code/digimon-engine-mcp/` | Read-only per-game engine forensics MCP server |
| `code/digimon-scenario-mcp/` | Dev/test MCP that stages, snapshots, and authors scenario tests |
| `code/digimon-training-mcp/` | Read-only MCP for inspecting `runs/` and `models/` training artifacts |
| `code/digimon_gym/digimon_gym.py` | `DigimonEnv` (Gymnasium interface) |
| `code/digimon_gym/agents/` | Pilot training, gauntlet, league, match env, reward profiles, anchored eval, champion registry, Elo ladder, exploiter, Architect agents, AlphaZero trainer |
| `code/digimon_gym/agents/maskable_recurrent/` | Custom recurrent PPO with action masking |
| `code/digimon_gym/inference/onnx_policy.py` | ONNX inference wrapper (no PyTorch) |
| `code/server/` | Hosted FastAPI app: `routers/` (games, lobby, rooms, matchmaking, WebSocket PvP, replays, deck tools), `db/`, `ai/`, `workers/`, `state_filter.py` |
| `code/src-tauri/` | Tauri v2 desktop shell: engine commands, ONNX sessions, deck tools, model cache, updater |
| `code/frontend/src/` | React UI |
| `code/landing/` | Static landing page |
| `code/tools/` | CLI tools: card ingest and bundles, DSL schema and lint, action-space codegen, `dcgo-replay`, `dcgo-harness`, archetype static tests, ONNX export, eval/Elo/exploiter CLIs (see [docs/TOOLS.md](docs/TOOLS.md)) |
| `code/tests/` | Default pytest tree: hosted API, RL, bindings surface, tools |
| `DCGO/` | Git submodule: modded DCGO C# client, the behavioral reference and recording oracle |
| `data/` | Card data (`cards.json`, `card_overrides.json`, `card_official.json`, `card_bundles/`), deck library, tested-cards allowlist |
| `configs/training/` | Training and eval-suite configs (`default.yaml`, `eval_suite.yaml`) |
| `openspec/` | OpenSpec change workflow: active `changes/<slug>/` plus `archive/` |
| `qa/` | Gap trackers, card verdicts, archetype QA, DCGO exam scenarios and verdicts |
| `docs/` | Reference documentation (start at [docs/INDEX.md](docs/INDEX.md)) |

## Quick Start

### Prerequisites

- Python 3.11+
- Node.js 18+ (frontend)
- Rust toolchain (engine, PyO3 bindings, Tauri)
- `maturin` (`pip install maturin`) to build the Python bindings

### Install

Pick the dependency set that matches what you're running, then install the project itself in editable mode. That step packages `server` and `digimon_gym` from `code/`.

```bash
pip install -r requirements.txt            # full hosted API (everything)
pip install -r requirements-training.txt   # training CLI (engine + torch/SB3, no FastAPI/DB)
pip install -e .

# Build the Rust engine bindings into the active Python env (required by
# DigimonEnv and the hosted API)
cd code/digimon-engine-py && maturin develop
```

The `DCGO/` submodule is a multi-GB Unity checkout. You only need it for card-behavior reference or oracle work:

```bash
git submodule update --init DCGO
```

### Rust Engine

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml

# Full per-card behavioral suite (large; this invocation is the reliable one)
RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml \
  --test cards_behavioral -- --test-threads=8

# DSL tooling for the YAML card specs
cargo run -p dsl-schema-export                       # regenerate the DSL JSON schema
cargo run -p dsl-lint -- code/digimon-engine/cards   # lint every card spec

# Debug REPL and recording viewer
cargo build -p digimon-engine-cli
target/debug/digimon-engine-cli debug
target/debug/digimon-engine-cli replay rec.json --step 47
```

### Hosted Backend

```bash
# Development (auto-reload)
python -m uvicorn server.api:app --reload --reload-dir code/server

# Production / long-running (no --reload; it leaves zombie watcher processes)
python -m uvicorn server.api:app --host 0.0.0.0 --port 8000
```

### Frontend

```bash
cd code/frontend
npm install
npm run dev            # web build
npm run dev:desktop    # desktop-mode build (admin/training UI excluded)
```

### Desktop App

The desktop build has no Python dependency. Gameplay, ONNX inference, and deck tools all run inside the embedded `digimon-engine` crate via Tauri `invoke()` commands. `tauri.conf.json` builds the desktop frontend automatically.

```bash
cd code/src-tauri
cargo tauri dev     # development
cargo tauri build   # installers land in the workspace-root target/release/bundle/
```

Releases are cut by tagging. See [docs/runbooks/desktop-release.md](docs/runbooks/desktop-release.md).

### Smoke-Check the RL Environment

```bash
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
```

The observation length depends on the active tensor profile (see [docs/TENSOR_SPEC.md](docs/TENSOR_SPEC.md)). The action mask is always 2192 wide.

### Run Tests

```bash
python -m pytest -v                          # default suite (testpaths = code/tests)
python -m pytest code/tests/api -v           # hosted API
python -m pytest code/tests/rl -v            # RL training
python -m pytest -m "not slow" -v            # skip slow smoke tests
python -m pytest code/tests/ai_pipeline -v   # admin AI pipeline (opt-in)

cd code/frontend && npm test                 # frontend unit tests (Vitest)
cd code/frontend && npm run e2e              # Playwright end-to-end tests
```

### Train an Agent

Training reads `configs/training/default.yaml`. Individual fields can be overridden with `--set key=value`.

```bash
python -m digimon_gym.agents.pilot_training --timesteps 500000
python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000
python -m digimon_gym.agents.pilot_training --archetypes rocks,ts-olympos --timesteps 500000

# Fictitious self-play against frozen champions (--self-play is retired)
python -m digimon_gym.agents.pilot_training --opponent pool \
  --opponent-pool-manifest pool.json --timesteps 1000000
```

Each episode is a best-of-three match by default. Pass `--match-format single` for one-game episodes. For cloud training (RunPod GPU or Hetzner/DigitalOcean CPU), see [docs/CLOUD_TRAINING.md](docs/CLOUD_TRAINING.md).

### Evaluate and Export Models

Don't rank models by the in-run win rate. It reuses the training opponent and is meaningless under self-play. Use anchored evaluation instead: play seat-balanced games against greedy and frozen champions (see [docs/MODEL_EVALUATION.md](docs/MODEL_EVALUATION.md)).

```bash
python code/tools/anchored_eval_cli.py --help

python code/tools/export_onnx.py --type mlp  --input models/mlp_agent.zip  --output models/mlp_agent.onnx
python code/tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx
```

## Authoring Cards

Card work is test-driven and DSL-first:

1. Write a failing `DebugRunner` test in `code/digimon-engine/tests/cards_behavioral/<set>/`, derived from the card's printed text.
2. Author the YAML spec in `code/digimon-engine/cards/<set>/` until the test passes.
3. If the DSL can't express the card, add the vocabulary in `code/digimon-dsl/` (log it to `qa/dsl-vocab-gaps.md`) or the engine primitive (log it to `docs/RUST_ENGINE_GAPS.md`). Hand-written `CardEffect`s in `src/cards/raw_rust/` are a last resort, and they must stay clone-safe.

When a question is about rules or card behavior, consult sources in this order: the official rules manual (`general_rule.pdf`), then DCGO's C# implementation, then the card-text JSON. When the question is about **printed card data** (traits, costs, text), the official Bandai card database wins; it is mirrored locally in `data/card_official.json` and `data/card_bundles/`. [CLAUDE.md](CLAUDE.md) has the full source-priority rules, plus the agent skills that automate archetype- and set-scale authoring (`/assess-archetype-rust`, `/batch-implement-cards-rust-dsl`, `/author-set`, `/archetype-campaign`).

## Documentation

Start at [docs/INDEX.md](docs/INDEX.md) for the full catalog.

| Document | Purpose |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Engineering guide: working rules, source priority, commands |
| [AGENTS.md](AGENTS.md) | RL agent architecture, wrapper chain, gauntlet orchestration |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | API surface, RL contracts, frontend components, desktop distribution |
| [docs/TENSOR_SPEC.md](docs/TENSOR_SPEC.md) | Observation tensor profiles and layouts |
| [docs/ACTION_SPEC.md](docs/ACTION_SPEC.md) | 2192-action space and phase-aware decoding |
| [docs/digimon-rules/](docs/digimon-rules/) | Verified rules derivations from the official manual: keyword semantics, digest, page index |
| [docs/RUST_ENGINE_API.md](docs/RUST_ENGINE_API.md) | Rust scripting API: `EffectContext`, `Effect`, `CardEffect`, deletion lifecycle |
| [docs/RUST_DSL_AGENT_GUIDE.md](docs/RUST_DSL_AGENT_GUIDE.md) | Authoring YAML DSL cards |
| [docs/RUST_DSL_TEST_API.md](docs/RUST_DSL_TEST_API.md) | Per-card and archetype interaction test patterns |
| [docs/RUST_ENGINE_GAPS.md](docs/RUST_ENGINE_GAPS.md) | Engine primitives still needed, by archetype |
| [docs/DCGO_RECORDING_SCHEMA.md](docs/DCGO_RECORDING_SCHEMA.md) | DCGO recording format consumed by `dcgo-replay` |
| [docs/DCGO_EXAM.md](docs/DCGO_EXAM.md) | Per-clause card exams against the DCGO oracle |
| [docs/DEBUG_MCP.md](docs/DEBUG_MCP.md) | Engine debug CLI and MCP, including replay stepping and scanners |
| [docs/TRAINING_RUNBOOK.md](docs/TRAINING_RUNBOOK.md) | Training pipeline operations |
| [docs/CLOUD_TRAINING.md](docs/CLOUD_TRAINING.md) | Cloud training on RunPod and Hetzner/DigitalOcean |
| [docs/MODEL_EVALUATION.md](docs/MODEL_EVALUATION.md) | Anchored eval, Elo ladder, champion registry, exploitability |
| [docs/REWARD_PROFILES.md](docs/REWARD_PROFILES.md) | Composable reward shaping |
| [docs/MODEL_CATALOG.md](docs/MODEL_CATALOG.md) | ONNX model upload/download pipeline and desktop cache |
| [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) | Hosted API deployment |
| [docs/ENVIRONMENT.md](docs/ENVIRONMENT.md) | Every environment variable, by subsystem |
| [docs/TOOLS.md](docs/TOOLS.md) | CLI tools reference |
| [docs/runbooks/](docs/runbooks/) | API deploy, desktop release, oracle node, starter-curriculum training |

## Roadmap

**Recently landed:**

- Rust engine as the sole engine. The Python engine was removed, and every surface runs on Rust.
- DSL-first card scripting with per-card TDD, archetype interaction tests, and whole-set authoring workflows.
- DCGO oracle pipeline: recording mod, `dcgo-replay` parity harness, unattended job harness, and per-clause card exams.
- `Game: Clone`, with determinized MCTS and an AlphaZero-style self-play loop.
- Anchored model evaluation, champion registry, Elo ladder, and pool/league opponents. In-engine self-play is retired.
- Best-of-three match episodes with concede and play-order selection.
- Python-free Tauri desktop app with runtime model downloads and a signed auto-updater.
- Room-based PvP, matchmaking, and spectating with hidden-information filtering.

**In progress / planned:**

- Grow card-pool coverage and per-clause oracle verification across archetypes and sets.
- Productionize search (leaf batching) and add game-review annotations.
- UI parity with the DCGO client: action log, card preview, per-card command panel, gameplay options (see [docs/UI_ROADMAP.md](docs/UI_ROADMAP.md)).
- Contextual card embeddings for recommending cards the deck builder hasn't seen.
- Alternative formats: EDEN deck legality, plus 4-player Commander/EDH and Titan modes. The engine is currently two-player.

Active work is tracked as OpenSpec changes under [openspec/changes/](openspec/changes/).

## Notes

- Keep observation and action specs, engine code, and tests in sync in the same change.
- Avoid hardcoding snapshot counts in docs. The implementation is the source of truth.
- Network clients must only receive state that has gone through `state_filter.py`. Never send raw state that would leak the opponent's hand.

## Credits

This is a fan-made, non-commercial simulator for playtesting and research. It is not affiliated with Bandai or Toei Animation. See [CREDITS.md](CREDITS.md) for trademark notices and attribution for bundled community art.
