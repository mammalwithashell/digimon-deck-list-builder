## Context

The `GameLog` component renders `store.logs: string[]` and shows "No log entries yet" because that buffer is never filled. The fill path is: FastAPI `_autoplay_agent_turns`/action handlers call `game.get_last_log()` → the PyO3 `RustHeadlessGame::get_last_log()` returns `PyList::empty_bound` ("empty until a future recording milestone ports the logger", `code/digimon-engine-py/src/lib.rs:775`) → response `logs: []` → `store.appendLogs([])`.

Meanwhile the engine's structured `GameEvent` stream IS fully populated (`engine-event-emission`), flows through the same responses (`events`), is appended to `store.events`, and already drives `DigivolveBanner`/`BattleEffect` animations. So a complete record of what happened is already in the client — just not as text.

## Goals / Non-Goals

**Goals:**
- A populated, scrolling in-game action log covering both players' actions.
- Reuse the existing event stream as the single source of truth.
- Readable lines with resolved card/player names and the existing clickable card-reference affordance.

**Non-Goals:**
- Un-stubbing `get_last_log()` or adding a parallel textual logger in the Rust engine.
- A canonical engine-side formatter shared across CLI/MCP/replay (possible future; see Decisions).
- Changing the `GameEvent` schema or the HTTP/Tauri response contract.
- Runtime card-effect serialization or the card preview (separate changes).

## Decisions

**1. Derive the log from `store.events` via a pure formatter (Option A), not the engine logger (B) or an event `summary` field (C).**

| Option | Where text is built | Engine/binding change | Reuse (CLI/MCP) | Speed | Risk |
|---|---|---|---|---|---|
| A. Frontend formatter | TS | none | no | fast | low |
| B. Engine textual logger | Rust | un-stub + new logger | yes | slow | medium |
| C. `summary` on each event | Rust | extend event schema | yes | medium | med-high (recording schema) |

Rationale: the events already reach the client and carry the data; a frontend formatter unblocks immediately with zero engine risk and keeps events as the one source of truth (the log is a derived view, not a second record). B/C duplicate the events' information as strings and touch the binding/event schema (and C affects recordings). If a future need arises for identical log text in non-TS surfaces (debug CLI, `digimon-engine-mcp`, replay viewer), the formatter can be lifted to a shared Rust function then — this change does not preclude it.

**2. Point `GameLog` at events through a derived selector, keep the component dumb.**
Add `utils/gameLogFormat.ts` exporting `formatEvent(event, ctx) => string[]` (and/or `formatEvents(events, state)`). `GameLog`/`GameLogDrawer` render the formatted lines from `store.events` (memoized selector) instead of `store.logs`. The existing `CARD_REF_PATTERN`/clickable-card-name rendering in `GameLog` is retained — the formatter emits `[CARD_ID:Name]` tokens so references stay clickable. *Alternative considered:* format into `store.logs` on each `appendEvents` — rejected to avoid a second stored copy that can drift from `events`.

**3. Resolve names from event payload first, then current state.**
The formatter resolves a referenced card/player using fields on the event; where an event references a board entity by handle/index, it falls back to current `player1`/`player2` battle-area/hand to get the name. Entities that have left all zones and aren't denormalized on the event are rendered by id or a neutral phrase rather than failing.

**4. Retire the dead textual `logs` path as cleanup, without breaking the contract.**
`store.logs`/`appendLogs` and the response `logs` field become vestigial once `GameLog` reads events. Remove the frontend `logs` store slice and `appendLogs` calls; leave the server `logs` response key present-but-empty (or drop it) per a quick check that nothing else consumes it. The binding `get_last_log()` stub can be left with its existing comment.

## Risks / Trade-offs

- **[An event lacks a field the log needs to name a card/player]** → Render a safe fallback (id or neutral text) and file the specific gap against `engine-event-emission` as a small, separate enhancement; do not block the log on it.
- **[Formatting drifts from animation semantics]** → Both derive from the same event; the spec requires the log entry and animation to come from the same event, keeping them consistent.
- **[Event volume makes the log noisy]** → The formatter omits player-irrelevant events (internal/sequence-only) and can group sub-events; tune the included set during implementation.
- **[Log/events ordering]** → `store.events` is appended in arrival order with sequence numbers; render in that order. No reordering needed.

## Migration Plan

Additive frontend change; no data or contract migration. Ship without a flag (view-only). Rollback = revert the frontend diff; if the `logs` slice was removed, the revert restores it. Verify via unit tests over `GameEvent` fixtures → expected lines and a manual/Playwright check that a real bot game yields a populated, scrolling, both-sides log.

## Open Questions

- Which `GameEvent` variants are in-scope for v1 (full coverage vs the common set: play, digivolve, attack, security check, effect activate/fizzle, memory, turn/phase, deletion, game over)?
- Do we keep the server `logs` response key for backward compatibility or remove it? (Lean: keep present-but-empty short-term, remove in a follow-up once confirmed unused.)
- Should the formatter live where it could later be promoted to a shared Rust implementation, or is the TS-only projection acceptable indefinitely? (Lean: TS-only now; revisit if a non-TS surface needs identical text.)
