## Why

The in-game action log is always empty ("No log entries yet"), so a player cannot review what happened during a turn — their own plays or the bot's. The Rust engine already emits a complete stream of structured `GameEvent`s (it reaches the frontend today and drives the digivolve/battle animations), but the textual log buffer the UI renders is fed by the PyO3 binding's `get_last_log()`, which is a stub that returns an empty list. The information exists; it just isn't projected into human-readable log lines.

## What Changes

- Render the in-game `GameLog` from the **structured event stream** (`store.events`) instead of the empty textual `logs` buffer.
- Add a pure **`GameEvent` → log line(s)** formatter that turns typed events (card played, digivolved, attack declared, security checked, effect activated/fizzled, memory change, turn/phase change, game over, …) into readable text, resolving card/player names from the event payload and current game state.
- Keep the engine and binding as the single source of truth: this change does **not** un-stub `get_last_log()` and does **not** add a parallel textual logger in the engine — the log is a projection of the canonical event stream.
- Mark the now-vestigial textual `logs` plumbing (server `get_last_log()` drain → response `logs` → `store.appendLogs`) as dead and simplify/remove it as cleanup, without changing the response contract relied on elsewhere.

Out of scope (separate changes):
- Runtime-accurate **card-effect state** in the stack inspector (engine `to_ui_json` text stubs) — see `surface-runtime-card-state`.
- The right-click **card preview** — see `add-ingame-card-preview`.
- A canonical engine-side log formatter shared with CLI/MCP/replay (documented as a future option in design).

## Capabilities

### New Capabilities
- `ingame-action-log`: A human-readable, scrolling action log during a game, derived from the engine's structured event stream and covering both players' actions.

### Modified Capabilities
<!-- None. The engine event stream already exists (engine-event-emission); this change consumes it in the UI. If a specific event is found to lack a field the log needs, that gap is filed against engine-event-emission separately. -->

## Impact

- **Frontend only** (primary): a new event→text formatter (e.g. `utils/gameLogFormat.ts`) and pointing `GameLog` at `store.events` (directly or via a derived selector). Components: `components/game/GameLog.tsx`, `components/game/GameLogDrawer.tsx`, `stores/gameStore.ts` (log/events wiring), `pages/GamePage.tsx`.
- Reuses the existing event flow already populated from create/action/step responses (`store.appendEvents`); no new network calls.
- No engine, binding, FastAPI, action-space, or game-state changes required. The binding `get_last_log()` stub may be left as-is or documented; the `logs` response field can be retired as cleanup.
- Dependency: the formatter relies on `GameEvent` payloads carrying enough denormalized data (ids/names) to render names; if any event is under-specified, that is a small, separately-tracked `engine-event-emission` enhancement, not part of this change's core.
- Verification: frontend unit tests (event fixtures → expected log lines) plus a manual/Playwright check that a real game produces a populated, scrolling log.
