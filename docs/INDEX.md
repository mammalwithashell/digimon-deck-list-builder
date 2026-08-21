# Documentation Index

> **Three bug-discovery modes** (all route confirmed findings to the same
> trackers — `docs/RUST_ENGINE_GAPS.md` for engine primitives,
> `qa/archetype-qa/engine-gaps.md` for card effects):
> 1. **Replay differential** — `/replay-bug-hunt` on a DCGO recording: the
>    engine is checked against the battle-tested DCGO oracle (a masked-out
>    recorded action is a Rust bug). *(reactive — see [DEBUG_MCP.md](DEBUG_MCP.md))*
> 2. **Replay judge** — `/replay-bug-hunt` on a native eval/self-play recording:
>    faithfulness judged vs card text + `general_rule.pdf` + DCGO C#. *(reactive)*
> 3. **Archetype probe** — `/archetype-interaction-test-author`: research an
>    archetype as a system, then author multi-card interaction tests + static
>    archetype tests that exercise its combos. *(proactive, hypothesis-driven —
>    see [RUST_DSL_TEST_API.md](RUST_DSL_TEST_API.md))*
>
> These sit **above** the per-card archetype family (`/assess-archetype-rust` →
> `/batch-implement-cards-rust-dsl` → per-card behavioral tests): they catch the
> cross-card / whole-game bugs per-card TDD can't see.

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Service architecture, API surface, frontend components, RL contracts, desktop distribution |
| [UI_ROADMAP.md](UI_ROADMAP.md) | UI roadmap toward DCGO parity for players — survey-based gap analysis (DCGO Unity client vs our React frontend), phased plan (bot pacing, per-card command panel, gameplay options/auto-processing, info access, sound), and the constraints that bind UI work |
| [ENVIRONMENT.md](ENVIRONMENT.md) | Environment variables — every var consumed by the hosted API, training CLI, desktop build, and frontend, grouped by subsystem |
| [TENSOR_SPEC.md](TENSOR_SPEC.md) | Observation tensor layout (1375 floats) |
| [ACTION_SPEC.md](ACTION_SPEC.md) | Action space (2192 actions) — ranges and conventions |
| [TRAINING_RUNBOOK.md](TRAINING_RUNBOOK.md) | RL training operations guide |
| [MODEL_EVALUATION.md](MODEL_EVALUATION.md) | How to evaluate models across training modes — anchored eval (greedy + frozen champions, seat-balanced), Elo ladder, champion registry, exploiter/exploitability, gated self-play, equilibrium-methods horizon |
| [REWARD_PROFILES.md](REWARD_PROFILES.md) | Composable reward shaping — YAML-defined per-archetype reward profiles, component catalog, budget engine, hot-reload, resume-hash check, telemetry reference, deprecation timeline for legacy flat shaping fields |
| [TOOLS.md](TOOLS.md) | CLI tools reference — card pipeline, transpiler, Pinecone, model export |
| [MODEL_CATALOG.md](MODEL_CATALOG.md) | ONNX model catalog — admin upload, desktop cache, storage backends, integrity |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Hosted API deployment — DigitalOcean topology, env vars, bootstrap |
| [RUST_ENGINE_API.md](RUST_ENGINE_API.md) | Rust engine scripting reference — `CardEffect`, `EffectContext`, `Expiry`, `ModifierType`, common patterns, DebugRunner |
| [SCENARIO_TESTING.md](SCENARIO_TESTING.md) | Scenario testing substrate — shared `qa/scenarios/*.json` fixtures, `RustDebugGame` + `/debug` staging, Rust + Playwright conformance runners |
| [DEBUG_MCP.md](DEBUG_MCP.md) | Engine debug MCP server + CLI — `digimon-engine-cli` REPL/replay, `digimon-engine-mcp` stdio tools (incl. recording stepping `step_forward`/`step_back`/`seek` + scanners `scan_divergences`/`scan_fizzles`/`scan_panics`), recipes. The interactive replay bug-hunter (`/replay-bug-hunt` skill) is the **microscope** that drives these tools; the [`dcgo-replay`](DCGO_RECORDING_SCHEMA.md) parity harness is the **funnel** that flags which recordings to point it at |
| [TRAINING_MCP.md](TRAINING_MCP.md) | Training status MCP — Python stdio server for read-only inspection of `runs/` and `models/` artifacts (list runs, summarize evals/panics, query TensorBoard metrics, inventory recordings/checkpoints/deck pool) |
| [SCENARIO_MCP.md](SCENARIO_MCP.md) | Scenario capture MCP (`digimon-scenario-mcp`) — **WRITE-capable dev/test** MCP that stages, snapshots, and authors game-state scenario tests over the `/debug` surface (browser) and the feature-gated Tauri debug bridge (desktop). The stage→capture→assert→emit-Playwright-spec loop, so a card/interaction can be UI-tested without playing a game to draw the right cards |
| [RUST_DSL_AGENT_GUIDE.md](RUST_DSL_AGENT_GUIDE.md) | Practical Rust YAML DSL authoring guide for agents — workflow, clause/step API, archetype patterns, gap filing, and tests |
| [RUST_DSL_TEST_API.md](RUST_DSL_TEST_API.md) | Rust DSL card test API — per-card behavioral test layout, DebugRunner helpers, DSL test patterns, and the **archetype interaction-test** bucket (`tests/archetypes/<slug>.rs` + `support.rs` fixtures) authored by the `/archetype-interaction-test-author` capstone skill. Its **static archetype-test harness** (deck-legality / coverage gate / smoke games / combo-presence) is the crate `code/tools/archetype-static-tests/` (`cargo run -p archetype-static-tests -- "<archetype>"`), recording verdicts in `qa/qa-reports/archetype_interactions.json` |
| [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md) | Rust ↔ Python engine parity tracker — every known behavioral divergence with severity and fix order |
| [RUST_ENGINE_GAPS.md](RUST_ENGINE_GAPS.md) | Rust engine capability gaps surfaced by archetype audits (`assess-rust-engine-archetype`) — primitives still needed before each archetype can ship under the no-approximations policy |
| [qa/dsl-vocab-gaps.md](../qa/dsl-vocab-gaps.md) | DSL vocabulary and lowering gaps surfaced by Rust archetype assessments and batch card implementation |
| [DCGO_KEYWORD_PARITY.md](DCGO_KEYWORD_PARITY.md) | DCGO ↔ Rust per-keyword behavioral parity tracker — every printed keyword cross-referenced against the C# source of truth |
| [DCGO_BUILD.md](DCGO_BUILD.md) | Building the modded DCGO client from source — Unity 2021.3.45f2 setup, asset-bundle acquisition, submodule pinning, bot-match smoke test |
| [DCGO_RECORDING_SCHEMA.md](DCGO_RECORDING_SCHEMA.md) | JSONL recording format produced by the DCGO mod and consumed by `dcgo-replay` — game_start / action / encoder_failure / reveal / game_end row shapes, opaque-mode `opp_decklist_composition`, schema versioning |
| [DCGO_EXAM.md](DCGO_EXAM.md) | DCGO card-clause exam — scripted scenarios (`qa/dcgo-exams/<SET>/<CARD-ID>.yaml`) that make DCGO answer "what does it do HERE" per printed clause; scenario format, the five verdict classes (`confirmed`/`diverged`/`unreachable`/`unavailable`/`unmeasured`, denominator always printed), the sim-only PR gate vs the local oracle pass, and the known gaps |
| [RULES_CONTEXT.md](RULES_CONTEXT.md) | Official Digimon TCG rules reference |
| [TITAN_MODE.md](TITAN_MODE.md) | Titan/Commander variant rules |
| [EDH_COMMANDER_MODE.md](EDH_COMMANDER_MODE.md) | EDH Commander mode design |
| [admin_ai_batch_runbook.md](admin_ai_batch_runbook.md) | AI batch processing runbook |

Related top-level docs:
- `CLAUDE.md` — project overview, tech stack, commands, working rules
- `AGENTS.md` — RL agent architecture, wrapper chain, gauntlet system

Related agent workflows:
- `.codex/skills/assess-rust-engine-archetype/` — Codex read-only DSL readiness assessment for archetypes, decks, card groups, or card lists
- `.claude/skills/assess-archetype-rust/` — legacy Claude gap-filing audit workflow that appends to `RUST_ENGINE_GAPS.md` and emits `.claude/plans/rust-engine-gaps-*.md`
