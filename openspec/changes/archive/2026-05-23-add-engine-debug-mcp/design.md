## Context

The Rust engine is the target source of truth (see [CLAUDE.md](../../../CLAUDE.md) "Rust pivot"). Agent workflows that today bottleneck on `cargo test`'s write-compile-run loop need a live, inspectable game surface. Three workflows converge on this need:

1. **Card debugging** — agents author or fix `CardEffect` impls and want to interrogate mid-game state.
2. **Smoke-test forensics** — flaky failures need `(decks, seed)` reproduction.
3. **Training-run forensics** — `GameRecorder` writes recordings on every training episode, but the Rust engine has no replay path, so the very recordings the training stack produces can't be inspected against the source-of-truth engine.

The Python side already solved (3) with `engine_py_legacy/engine/runners/replay_runner.py`. The Rust side has the `GameRecorder` ([recorder.rs](../../../code/digimon-engine/src/recorder.rs)) but no `ReplayRunner` — a parity gap independent of any agent-tooling question.

`load_implemented_card_ids()` already exists in [code/digimon-engine-py/src/lib.rs:574](../../../code/digimon-engine-py/src/lib.rs:574) and is the universal "what cards work?" filter used by `pilot_training`, `gauntlet`, and the architect agents. This change adopts the same filter as the default card pool for the new debug surface.

Snapshot/restore (branching debug — "what if I had picked differently?") is desirable but requires `Game: Clone`, which is blocked by `Box<dyn GameLogger>` (line 327) and by the >2 MB cost of cloning `card_data` ([line 207](../../../code/digimon-engine/src/game.rs:207)) plus the various registries. Making clones cheap means `Arc`-wrapping immutable shared state — a refactor that touches many accessors. This change explicitly defers that refactor and the branching tools that depend on it.

## Goals / Non-Goals

**Goals:**

- A single `LiveGame` abstraction in `digimon-engine` constructable from four sources (decks+seed, debug hands, recording, recording-at-step). Identical post-construction surface regardless of origin.
- Rust port of `ReplayRunner` closing the Python/Rust parity gap.
- View serialization layer that is **not** `to_ui_json` — debugging needs modifier/effect-queue/owner-controller detail that the frontend view drops.
- Two consumers: a `digimon-engine-cli` binary for humans and CI, a `digimon-engine-mcp` stdio server for AI agents. Both link the engine crate directly. Neither depends on the other.
- Default card pool = `load_implemented_card_ids()`. `--all-cards` escape hatch for replays referencing unimplemented cards.

**Non-Goals:**

- **Branching (`snapshot`/`restore`/`list_snapshots`).** Deferred to v1.5 once `Game: Clone` is cheap.
- **Engine refactor to `Arc`-wrap shared state.** Required for v1.5; out of scope here.
- **Training-worker integration that flushes recordings on engine panic.** A separate proposal; today recordings are flushed at episode boundaries, which already covers most crash cases.
- **Skill wrappers (`/debug-card`, `/investigate-crash`).** These will consume the MCP but are skill-authoring work outside this change.
- **Replacing `cargo test` for behavioral validation.** Tests-as-code remain the regression vehicle. The MCP/CLI are interactive complements.
- **Web UI for the debugger.** CLI + MCP is the entire surface.

## Decisions

### Decision 1: MCP server is a Rust binary, not a Python wrapper around PyO3

**Choice:** Build `digimon-engine-mcp` as a Rust binary in the workspace linking `digimon-engine` directly.

**Why:**
- No FFI round-trip on every tool call. Tool latency matters when an agent calls `state` 20 times per debug session.
- Same crate means the server can use `LiveGame`'s typed view structs and serialize them with `serde` — no JSON-from-Python intermediate representation.
- Avoids a Python runtime dependency for the agent tooling, matching the Tauri-side decision (Working Rule 8).

**Alternatives considered:**
- Python binary using `digimon-engine-py`'s `RustHeadlessGame`. Faster to prototype but introduces a PyO3 dependency and an FFI hop per call.
- TypeScript/Node MCP server shelling to a Rust binary per call. Stateless across calls — defeats the whole point.

**Risk:** Rust MCP SDK landscape is younger than Python's. Mitigation: see Decision 2.

### Decision 2: Use `rmcp` (Rust MCP SDK) if mature; fall back to hand-rolled stdio JSON-RPC if not

**Choice:** Evaluate `rmcp` (or whichever Anthropic-blessed Rust MCP crate is current at implementation time). If it covers stdio + tool registration + JSON schema, use it. Otherwise, implement a minimal stdio JSON-RPC loop directly — MCP over stdio is line-delimited JSON-RPC 2.0; the protocol surface needed here is small (initialize, tools/list, tools/call).

**Why:** Don't block on SDK maturity. The protocol is small enough to hand-roll if needed. The MCP server contract is the tool schema, not the SDK choice.

**Resolved during implementation.** Open question (see Open Questions).

### Decision 3: New `LiveGame` type, not extending `DebugRunner`

**Choice:** Introduce a new `LiveGame` struct in `digimon-engine` and keep `DebugRunner` unchanged.

**Why:**
- `DebugRunner` is `#[cfg(any(test, feature = "test-helpers"))]`-decorated heavily; it includes test-only helpers (`transfer_control`, `push_source_owned`) that should not leak into production builds.
- `DebugRunner` is the test ergonomics surface. `LiveGame` is the runtime surface. Different audiences, different stability commitments.
- `LiveGame` owns the recording-load construction path, which `DebugRunner` does not.

**Alternatives considered:** Promote `DebugRunner` to non-test. Rejected — it would expose test-internal helpers to runtime consumers.

### Decision 4: View serialization is a new module, not reused `to_ui_json`

**Choice:** Add `digimon_engine::view` exposing `StateView`, `HandView`, `FieldView`, `SecurityView`, `PendingSelectionView`, `EffectQueueView`, `ModifierView`, `EventLogView`. Each is a plain `serde::Serialize` struct.

**Why:**
- `to_ui_json` is opinionated for the frontend: redacts opponent hand metadata per Working Rule 14, omits modifier details the frontend doesn't render, embeds frontend-specific shapes (display strings, color hints).
- A debugger needs the opposite — full modifier list, owner vs controller, granted effect bodies, effect queue ordering.
- Separating the views keeps `to_ui_json` free to evolve for the frontend without breaking the debug surface.

**Stability:** Once published, field names are a wire contract. Renames require a spec delta per `engine-debug-mcp` "Tool Surface Stability."

### Decision 5: Perspective filtering happens in views, not at server boundary

**Choice:** Every view function takes a `Perspective` enum (`Player(PlayerId)`, `God`). Filtering opponent-hidden info is the view layer's responsibility.

**Why:**
- The MCP server's `view` parameter delegates to the same code the CLI uses — one filter implementation, two consumers.
- Mirrors `state_filter.py`'s role in the hosted API: untrusted callers ask for a player view; the server redacts. Same shape, same security invariant.

**Risk:** Forgetting to redact a field is a leak. Mitigation: god view is the unfiltered default; player view derives from god view by stripping fields. A property test ensures every player-view JSON is a subset of god-view JSON.

### Decision 6: Card pool default = `load_implemented_card_ids()`

**Choice:** When no `--pool` flag is given, both CLI and MCP load `card_data` filtered to the set returned by `digimon_engine::cards::build_registry().registered_card_ids()`.

**Why:**
- Same filter used by `pilot_training`, `gauntlet`, architect agents. Single source of truth — when a card lands in the registry, it lights up everywhere.
- Cuts `card_data` from ~4085 entries to ~300-500. Brings naive `Game` size down >10×.
- Recordings produced by training always reference this set (training itself uses the same filter), so the default works for forensic replay too.

**Escape hatch:** `--pool=all` loads the full `cards.json`. `--pool=path.json` loads a caller-supplied list.

**Risk:** A user replays a recording that referenced an unimplemented card — fails to load. Mitigation: error message names the missing cards and suggests `--all-cards`.

### Decision 7: Branching deferred

**Choice:** Ship v1 without `snapshot`/`restore`/`list_snapshots`. v1.5 adds them after the `Arc`-wrap refactor.

**Why:**
- `Game: Clone` is blocked by `Box<dyn GameLogger>` and >2 MB `card_data`. Hand-implementing Clone is doable but the deeper issue is that snapshots are slow without `Arc`-wrapping shared state.
- The `Arc`-wrap refactor is its own scope: touches every `&self.card_data` accessor and every test that mutates registries. Bundling it with this change inflates risk.
- Forward-only "load recording, seek to step N" covers most debug needs. Snapshots are the polish, not the foundation.

**Trigger for v1.5:** First real use of the MCP that hits a "I want to try the other line" wall. We'll learn from v1 usage whether snapshots are urgent or a "nice to have."

### Decision 8: Workspace layout — two new crates, not new binaries inside `digimon-engine`

**Choice:** Add two new workspace members:
- `code/digimon-engine-cli/` (binary crate, `name = "digimon-engine-cli"`)
- `code/digimon-engine-mcp/` (binary crate, `name = "digimon-engine-mcp"`)

Both depend on `digimon-engine` via `path`.

**Why:**
- `digimon-engine` is a library. Bolting binaries onto a library crate works but conflates dependency graphs (CLI needs `clap`, MCP needs JSON-RPC machinery — neither belongs in the engine's deps).
- Separate crates compile in parallel and keep the engine's compile graph clean for downstream consumers (`digimon-engine-py`, `src-tauri`).
- Pattern matches what already exists for `tools/dsl-schema-export` and `tools/dsl-lint`.

## Risks / Trade-offs

- **Risk:** View serialization grows expensive on large field states. → Mitigation: views are computed on demand, not cached; agents query specific views (e.g., just `pending_selection`) rather than `state` every time. Benchmark in implementation if `state` JSON exceeds ~20 KB.
- **Risk:** Rust MCP SDK immaturity forces hand-roll. → Mitigation: protocol is small; budget 1-2 days for hand-roll if needed (see Decision 2).
- **Risk:** Recording format drift between Python and Rust. → Mitigation: `recorder.rs` already mirrors the Python schema field-for-field; the Rust `ReplayRunner` consumes the same JSON. Add a parity test that loads a Python-recorded JSON, replays in Rust, and asserts state matches.
- **Risk:** Agents over-rely on the MCP and stop writing test code. → Mitigation: MCP is an exploration tool; `cargo test`-based behavioral tests remain the regression vehicle and are required by the no-approximations policy (Working Rules 17, 18).
- **Risk:** GameId enumeration in `list_games` leaks across sessions if the server is shared. → Mitigation: stdio servers are per-client; GameIds live in the server process and die when the client disconnects. Document this as expected.
- **Trade-off:** No branching in v1 means "what if" debugging requires fully re-replaying. Accepted; cited as v1.5 motivator.
- **Trade-off:** Two binaries instead of one. Slight build/distribution overhead; pays back in dependency cleanliness.

## Migration Plan

No migration needed — all work is additive. No existing code paths change. Rollback is `git revert`.

Once v1 lands:
1. Update `.mcp.json` to register `digimon-engine-mcp`.
2. Update `docs/RUST_PYTHON_PARITY.md` to mark `ReplayRunner` parity gap resolved.
3. Add `docs/DEBUG_MCP.md` (or a section in `RUST_ENGINE_API.md`) with tool reference and recipes.

When v1.5 ships (separate change):
1. Spec delta on `live-game-surface` adding snapshot/restore methods.
2. Spec delta on `engine-debug-mcp` adding branching tools.
3. Engine refactor (`Arc`-wrap card_data/registries) as prerequisite.

## Open Questions

1. **Rust MCP SDK choice.** Resolve in implementation: evaluate `rmcp`, `mcp-rust-sdk`, or whichever is current. Fall back to hand-rolled stdio JSON-RPC if needed. Either path satisfies the spec.
2. **GameId format.** UUID v4 (16 bytes, opaque) vs short token (e.g., 8 chars, human-friendly in logs). Recommendation: short token; debug session logs are easier to read.
3. **YAML scenario format compatibility.** The existing `tools/run_scenario.py` consumes a specific YAML shape. The Rust `scenario` subcommand should accept the same shape for migration ease, but the Python-side `ScenarioRunner` has Python-specific assertion helpers. Decide during implementation: full parity, or Rust-flavored variant. Recommendation: parity for the common subset, document divergences.
4. **`inspect_card` script_path resolution.** Implemented Rust cards have a known module path under `code/digimon-engine/src/cards/`. DSL cards have YAML paths under `code/digimon-engine/cards/`. Decide whether to return both, or whichever exists. Recommendation: return both fields, each optional.
5. **Recording schema versioning.** If the Rust recorder schema diverges from Python's, the replay path needs a version field. Today they match; check whether v1 lands with a version stamp on new recordings to enable future evolution.
