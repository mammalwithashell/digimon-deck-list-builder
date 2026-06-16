## Context

The `GameLog` component renders `store.logs: string[]` and shows "No log entries yet" because that buffer is never filled. The browser fill path is: FastAPI `_autoplay_agent_turns`/action handlers call `game.get_last_log()` -> the PyO3 `RustHeadlessGame::get_last_log()` returns `PyList::empty_bound` ("empty until a future recording milestone ports the logger", `code/digimon-engine-py/src/lib.rs:775`) -> response `logs: []` -> `store.appendLogs([])`.

Meanwhile the engine's structured `GameEvent` stream is the authoritative gameplay event source. In the browser/PyO3 path it flows through the same responses (`events`) and can be appended to `store.events`; in the desktop/Tauri path the response DTO already has `events` but `drain_events` is stubbed to `Vec::new()`. Existing animation code also expects legacy lowercase event names (`digivolve`, `battle_result`, `security_reveal`, `effect_activate`) while the Rust/PyO3 typed event bridge emits PascalCase names (`Digivolve`, `Attack`, `SecurityReveal`, `EffectFizzled`). The action log should normalize these shapes and render from the normalized event stream.

## Goals / Non-Goals

**Goals:**
- A populated, scrolling in-game action log covering both players' actions.
- Reuse the existing event stream as the single source of truth.
- Readable lines with resolved card/player names and the existing clickable card-reference affordance.
- Parity across browser/PyO3 and desktop/Tauri response paths.

**Non-Goals:**
- Un-stubbing `get_last_log()` or adding a parallel textual logger in the Rust engine.
- A canonical engine-side formatter shared across CLI/MCP/replay (possible future; see Decisions).
- Changing the HTTP/Tauri response contract shape.
- Adding new action-space or tensor contract surface.
- Runtime card-effect serialization or the card preview (separate changes).

## Decisions

**1. Derive the log from normalized `store.events` via a pure formatter (Option A), not the engine logger (B) or an event `summary` field (C).**

| Option | Where text is built | Engine/binding change | Reuse (CLI/MCP) | Speed | Risk |
|---|---|---|---|---|---|
| A. Frontend formatter | TS | none | no | fast | low |
| B. Engine textual logger | Rust | un-stub + new logger | yes | slow | medium |
| C. `summary` on each event | Rust | extend event schema | yes | medium | med-high (recording schema) |

Rationale: the events already exist and carry the data; a frontend formatter keeps events as the one source of truth (the log is a derived view, not a second record). B/C duplicate the events' information as strings and touch the binding/event schema (and C affects recordings). The one non-frontend exception is desktop event draining: Tauri must convert the existing Rust `Game::drain_events()` output into the already-defined `GameEventDto` response field so the shipping desktop app has the same event stream as browser/PyO3. If a future need arises for identical log text in non-TS surfaces (debug CLI, `digimon-engine-mcp`, replay viewer), the formatter can be lifted to a shared Rust function then - this change does not preclude it.

**1a. Normalize event type names at ingestion.**
Add a small `normalizeGameEvent` helper and run every response event list through it in `gameApi.ts` before appending to the store. Canonical frontend names stay aligned with the existing animation components (`digivolve`, `attack`, `security_reveal`, `memory_change`, `turn_start`, `phase_change`, `play`, `trash`, `mill`, `game_over`, `concede`, `effect_fizzled`). Legacy lowercase names pass through; Rust/PyO3 PascalCase names map to the canonical lowercase names. Animation consumers should use the canonical names, with aliases retained only where needed during migration.

**2. Point `GameLog` at events through a derived selector, keep the component dumb.**
Add `utils/gameLogFormat.ts` exporting `formatEvent(event, ctx) => string[]` (and/or `formatEvents(events, state)`). `GameLog`/`GameLogDrawer` render the formatted lines from `store.events` (memoized selector) instead of `store.logs`. The existing `CARD_REF_PATTERN`/clickable-card-name rendering in `GameLog` is retained — the formatter emits `[CARD_ID:Name]` tokens so references stay clickable. *Alternative considered:* format into `store.logs` on each `appendEvents` — rejected to avoid a second stored copy that can drift from `events`.

**3. Resolve names from event payload first, then current state.**
The formatter resolves a referenced card/player using fields on the event; where an event references a board entity by handle/index, it falls back to current `player1`/`player2` battle-area/hand to get the name. Entities that have left all zones and aren't denormalized on the event are rendered by id or a neutral phrase rather than failing.

**4. Retire the dead textual `logs` path as frontend cleanup, without breaking the wire contract.**
`store.logs`/`appendLogs` become vestigial once `GameLog` reads events. Remove the frontend `logs` store slice and `appendLogs` calls. Keep the `logs` response key present-but-empty short-term on browser and desktop wires because the API types already include it and removing it buys little. The binding `get_last_log()` stub can be left with its existing comment.

**5. Preserve create-response events.**
The browser route intentionally includes opening bot prelude events in the `POST /games` response. `gameApi.createGame()` must expose those events and `GamePage` must append them on create, just like action/step responses. Desktop create currently has no agent prelude and may return no events, but the shape should allow them for parity.

## Risks / Trade-offs

- **[An event lacks a field the log needs to name a card/player]** → Render a safe fallback (id or neutral text) and file the specific gap against `engine-event-emission` as a small, separate enhancement; do not block the log on it.
- **[Formatting drifts from animation semantics]** -> Normalize event type names once at ingestion and make both formatter and animations consume the canonical names, keeping log entries and animations tied to the same event.
- **[Desktop stays empty despite frontend work]** -> Tauri `drain_events` is in scope and gets a focused Rust unit test/smoke assertion for converting at least memory/play/digivolve/game-over events into `GameEventDto`.
- **[Event volume makes the log noisy]** → The formatter omits player-irrelevant events (internal/sequence-only) and can group sub-events; tune the included set during implementation.
- **[Log/events ordering]** → `store.events` is appended in arrival order with sequence numbers; render in that order. No reordering needed.

## Migration Plan

Additive frontend + desktop response implementation; no data or contract migration. Ship without a flag (view-only). Rollback = revert the frontend/Tauri diff; if the `logs` slice was removed, the revert restores it. Verify via unit tests over `GameEvent` fixtures -> expected lines, event-normalization tests, a Tauri conversion test, and a manual/Playwright check that a real bot game yields a populated, scrolling, both-sides log.

## Open Questions

- Which `GameEvent` variants are in-scope for v1? Decision: support the Rust/PyO3 emitted set (`MemoryChange`, `TurnStart`, `PhaseChange`, `Play`, `Digivolve`, `Attack`, `Trash`, `Mill`, `SecurityReveal`, `GameOver`, `Concede`, `EffectFizzled`) plus legacy lowercase aliases already used by animations.
- Do we keep the server `logs` response key for backward compatibility or remove it? Decision: keep present-but-empty short-term; remove only in a later wire-cleanup change.
- Should the formatter live where it could later be promoted to a shared Rust implementation, or is the TS-only projection acceptable indefinitely? (Lean: TS-only now; revisit if a non-TS surface needs identical text.)
