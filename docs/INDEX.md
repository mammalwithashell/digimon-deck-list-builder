# Documentation Index

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Service architecture, API surface, frontend components, RL contracts, desktop distribution |
| [ENVIRONMENT.md](ENVIRONMENT.md) | Environment variables — every var consumed by the hosted API, training CLI, desktop build, and frontend, grouped by subsystem |
| [TENSOR_SPEC.md](TENSOR_SPEC.md) | Observation tensor layout (1375 floats) |
| [ACTION_SPEC.md](ACTION_SPEC.md) | Action space (2192 actions) — ranges and conventions |
| [TRAINING_RUNBOOK.md](TRAINING_RUNBOOK.md) | RL training operations guide |
| [TOOLS.md](TOOLS.md) | CLI tools reference — card pipeline, transpiler, Pinecone, model export |
| [MODEL_CATALOG.md](MODEL_CATALOG.md) | ONNX model catalog — admin upload, desktop cache, storage backends, integrity |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Hosted API deployment — DigitalOcean topology, env vars, bootstrap |
| [RUST_ENGINE_API.md](RUST_ENGINE_API.md) | Rust engine scripting reference — `CardEffect`, `EffectContext`, `Expiry`, `ModifierType`, common patterns, DebugRunner |
| [DEBUG_MCP.md](DEBUG_MCP.md) | Engine debug MCP server + CLI — `digimon-engine-cli` REPL/replay, `digimon-engine-mcp` stdio tools, recipes |
| [TRAINING_MCP.md](TRAINING_MCP.md) | Training status MCP — Python stdio server for read-only inspection of `runs/` and `models/` artifacts (list runs, summarize evals/panics, query TensorBoard metrics, inventory recordings/checkpoints/deck pool) |
| [RUST_DSL_AGENT_GUIDE.md](RUST_DSL_AGENT_GUIDE.md) | Practical Rust YAML DSL authoring guide for agents — workflow, clause/step API, archetype patterns, gap filing, and tests |
| [RUST_DSL_TEST_API.md](RUST_DSL_TEST_API.md) | Rust DSL card test API — per-card behavioral test layout, DebugRunner helpers, and DSL test patterns |
| [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md) | Rust ↔ Python engine parity tracker — every known behavioral divergence with severity and fix order |
| [RUST_ENGINE_GAPS.md](RUST_ENGINE_GAPS.md) | Rust engine capability gaps surfaced by archetype audits (`assess-rust-engine-archetype`) — primitives still needed before each archetype can ship under the no-approximations policy |
| [qa/dsl-vocab-gaps.md](../qa/dsl-vocab-gaps.md) | DSL vocabulary and lowering gaps surfaced by Rust archetype assessments and batch card implementation |
| [DCGO_KEYWORD_PARITY.md](DCGO_KEYWORD_PARITY.md) | DCGO ↔ Rust per-keyword behavioral parity tracker — every printed keyword cross-referenced against the C# source of truth |
| [DCGO_BUILD.md](DCGO_BUILD.md) | Building the modded DCGO client from source — Unity 2021.3.45f2 setup, asset-bundle acquisition, submodule pinning, bot-match smoke test |
| [DCGO_RECORDING_SCHEMA.md](DCGO_RECORDING_SCHEMA.md) | JSONL recording format produced by the DCGO mod and consumed by `dcgo-replay` — game_start / action / encoder_failure / reveal / game_end row shapes, opaque-mode `opp_decklist_composition`, schema versioning |
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
