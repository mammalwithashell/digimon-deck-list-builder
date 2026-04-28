# Documentation Index

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Service architecture, API surface, frontend components, RL contracts, desktop distribution |
| [ENVIRONMENT.md](ENVIRONMENT.md) | Environment variables — every var consumed by the hosted API, training CLI, desktop build, and frontend, grouped by subsystem |
| [TENSOR_SPEC.md](TENSOR_SPEC.md) | Observation tensor layout (1375 floats) |
| [ACTION_SPEC.md](ACTION_SPEC.md) | Action space (2168 actions) — ranges and conventions |
| [TRAINING_RUNBOOK.md](TRAINING_RUNBOOK.md) | RL training operations guide |
| [TOOLS.md](TOOLS.md) | CLI tools reference — card pipeline, transpiler, Pinecone, model export |
| [RUST_ENGINE_API.md](RUST_ENGINE_API.md) | Rust engine scripting reference — `CardEffect`, `EffectContext`, `Expiry`, `ModifierType`, common patterns, DebugRunner |
| [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md) | Rust ↔ Python engine parity tracker — every known behavioral divergence with severity and fix order |
| [RUST_ENGINE_GAPS.md](RUST_ENGINE_GAPS.md) | Rust engine capability gaps surfaced by archetype audits (`/assess-archetype-rust`) — primitives still needed before each archetype can ship under the no-approximations policy |
| [qa/dsl-vocab-gaps.md](../qa/dsl-vocab-gaps.md) | DSL vocabulary and lowering gaps surfaced by Rust archetype assessments and batch card implementation |
| [DCGO_KEYWORD_PARITY.md](DCGO_KEYWORD_PARITY.md) | DCGO ↔ Rust per-keyword behavioral parity tracker — every printed keyword cross-referenced against the C# source of truth |
| [RULES_CONTEXT.md](RULES_CONTEXT.md) | Official Digimon TCG rules reference |
| [TITAN_MODE.md](TITAN_MODE.md) | Titan/Commander variant rules |
| [EDH_COMMANDER_MODE.md](EDH_COMMANDER_MODE.md) | EDH Commander mode design |
| [UI_PLAN.md](UI_PLAN.md) | UI design plan |
| [admin_ai_batch_runbook.md](admin_ai_batch_runbook.md) | AI batch processing runbook |

Related top-level docs:
- `CLAUDE.md` — project overview, tech stack, commands, working rules
- `AGENTS.md` — RL agent architecture, wrapper chain, gauntlet system
