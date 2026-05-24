## Why

`LiveGame` and the `digimon-engine-mcp` server were built so AI agents, QA harnesses, and human debuggers could drive the Rust engine through structured tool calls. In practice the action surface silently swallows invalid input, lets actions execute out of turn or out of phase, advertises legal actions for inactive players, omits two spec'd tools entirely, and serializes events as Rust `Debug` strings. A scripted MCP-driven Medusamon vs Puppets QA session ([qa/qa-reports/2026-05-23-medusamon-vs-puppets-mcp.md](qa/qa-reports/2026-05-23-medusamon-vs-puppets-mcp.md)) confirmed eight violations of the `live-game-surface` and `engine-debug-mcp` contracts, all in `code/digimon-engine/src/live_game.rs` and `code/digimon-engine/src/events.rs`. The bugs cascade: silent no-ops on illegal action IDs combine with phantom legal-action listings to make scripted play loops invisibly stall, and the Debug-string event format forces every observer to regex-parse what the spec promises will be structured data.

## What Changes

- `LiveGame::step` returns `ActionResult { ok: false, error: "..." }` when the supplied `action_id` is not legal for `current_decision_player()` (no state change, no events).
- `LiveGame::play` validates `current_decision_player() == player` and rejects when the active phase is not `Main` (covers the observed mulligan / opponent-turn corruptions).
- `LiveGame::end_turn` and `LiveGame::pass_turn` reject when called outside legal phases (no silent fast-forward through Mulligan).
- `LiveGame::legal_actions(player)` returns an empty `Vec` when `player != current_decision_player()`, so harnesses cannot iterate phantom actions for the inactive player.
- The engine never exposes a mandatory pending selection whose only option is unfulfillable (observed in a DNA Omnimon vs BG Imperial scripted run: BT17-081's `[End of Your Turn]` "Select 1 of your [Omnimon]-named Digimon" with zero matching targets surfaces a single `step`-no-op option, causing a hard soft-lock). The engine SHALL either fizzle the effect (clear pending, emit a fizzle event, advance) or always include a pass / decline action.
- `GameEvent` and the types it contains derive `serde::Serialize`; `LiveGame::make_result` and the MCP `events`, `step`, `play`, etc. tools emit structured event objects instead of `format!("{:?}", e)` strings.
- Add the `digivolve` and `attack` MCP tools the `engine-debug-mcp` spec already promises (currently a v1 "use `step`" workaround acknowledged in `docs/DEBUG_MCP.md` but not in the spec).
- Tighten spec scenarios in `live-game-surface` and `engine-debug-mcp` to make the validation and serialization contracts explicit.
- Use the in-process Rust integration-test layer (`code/digimon-engine/tests/`) plus a stdio JSON-RPC fixture against `digimon-engine-mcp` to regression-test every scenario.

## Capabilities

### New Capabilities

None — this proposal repairs the existing `live-game-surface` and `engine-debug-mcp` capabilities.

### Modified Capabilities

- `live-game-surface` — Action methods enforce phase and decision-player validation; `legal_actions` is gated on the active decision player; `events_emitted` is structured.
- `engine-debug-mcp` — Tool surface adds `digivolve` and `attack`; every action tool returns structured errors on illegal calls; `events` and `events_emitted` payloads contain typed `GameEvent` objects.

## Impact

- **Affected Rust engine code**: `code/digimon-engine/src/live_game.rs` (validation gates, structured event serialization), `code/digimon-engine/src/events.rs` (derive `Serialize` on `GameEvent` and contained types), `code/digimon-engine-mcp/src/tools.rs` (two new tool handlers + pass-through of structured events).
- **Affected tests**: `code/digimon-engine/tests/` gets new integration tests for each scenario. A small Python integration fixture under `.claude/tmp/` becomes `code/digimon-engine-mcp/tests/` end-to-end coverage.
- **Wire-format breaking change**: any current MCP consumer that regex-parses the Debug-format event strings will need to update. Today the only known consumers are the QA scripts under `.claude/tmp/` and the docs examples; the RL training pipeline does not consume `events_emitted` (it goes through `HeadlessRunner`, which has no return).
- **HeadlessRunner contract preserved**: `HeadlessRunner::step` keeps its fire-and-forget signature; the RL pipeline is unaffected.
- **PyO3 bindings**: `code/digimon-engine-py/src/lib.rs` does not currently expose `LiveGame` action methods, so the change is invisible to Python callers of `RustHeadlessGame`.
- **No tensor / action-space / model changes**: `ACTION_SPACE_SIZE`, observation profiles, and all tensor shapes are unchanged. No model retraining.
- **Reference dependency**: DCGO is initialized in this worktree but is not used as a behavioral reference for this change (DCGO is a Unity client without an analogous low-level action API). It remains available for follow-on card-effect QA.
