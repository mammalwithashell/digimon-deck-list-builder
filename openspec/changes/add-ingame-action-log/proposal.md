## Why

The in-game action log is always empty ("No log entries yet"), so a player cannot review what happened during a turn - their own plays or the bot's. The Rust engine already emits structured `GameEvent`s and the browser/PyO3 path drains them into the frontend, but the textual log buffer the UI renders is fed by `get_last_log()`, which is a stub that returns an empty list. The desktop/Tauri path has the response field but currently returns an empty event vector. The information exists in the engine; it just isn't projected consistently into human-readable log lines.

## What Changes

- Render the in-game `GameLog` from the **structured event stream** (`store.events`) instead of the empty textual `logs` buffer.
- Add a pure **`GameEvent` -> log line(s)** formatter that turns typed events (card played, digivolved, attack declared, security checked, effect activated/fizzled, memory change, turn/phase change, game over, ...) into readable text, resolving card/player names from the event payload and current game state.
- Normalize event type names at the frontend boundary so legacy lowercase animation names and Rust/PyO3 PascalCase event names are consumed through one canonical event vocabulary.
- Preserve event payloads from create responses, including the browser path's opening bot prelude, so the log does not miss actions that happen before the first local decision.
- Wire the desktop/Tauri event drain from the in-process Rust `Game` into the existing `events` response field; keep the wire shape unchanged.
- Keep the engine and binding as the single source of truth: this change does **not** un-stub `get_last_log()` and does **not** add a parallel textual logger in the engine — the log is a projection of the canonical event stream.
- Mark the now-vestigial textual `logs` plumbing (server `get_last_log()` drain → response `logs` → `store.appendLogs`) as dead and simplify/remove it as cleanup, without changing the response contract relied on elsewhere.

Out of scope (separate changes):
- Runtime-accurate **card-effect state** in the stack inspector (engine `to_ui_json` text stubs) - see `add-permanent-stack-inspector` (`surface-runtime-card-state` was superseded/cancelled).
- The right-click **card preview** — see `add-ingame-card-preview`.
- A canonical engine-side log formatter shared with CLI/MCP/replay (documented as a future option in design).

## Capabilities

### New Capabilities
- `ingame-action-log`: A human-readable, scrolling action log during a game, derived from the engine's structured event stream and covering both players' actions.

### Modified Capabilities
<!-- None. The engine event stream already exists (engine-event-emission); this change consumes it in the UI. If a specific event is found to lack a field the log needs, that gap is filed against engine-event-emission separately. -->

## Impact

- **Frontend** (primary): a new event normalizer + event-to-text formatter (e.g. `utils/gameEvents.ts`, `utils/gameLogFormat.ts`) and pointing `GameLog` at `store.events` (directly or via a derived selector). Components: `components/game/GameLog.tsx`, `components/game/GameLogDrawer.tsx`, `stores/gameStore.ts` (log/events wiring), `pages/GamePage.tsx`, `api/gameApi.ts`.
- **Desktop/Tauri**: implement the existing `drain_events` response plumbing in `code/src-tauri/src/engine_commands.rs` by converting drained Rust `GameEvent`s into the existing `GameEventDto` shape.
- Reuses the existing event flow already populated from create/action/step responses (`store.appendEvents`); no new network calls.
- No action-space, tensor, game-state, FastAPI schema, or PyO3 textual-log changes required. The binding `get_last_log()` stub may be left as-is or documented; the `logs` response field can be retained present-but-empty for compatibility.
- Dependency: the formatter relies on `GameEvent` payloads carrying enough denormalized data (ids/names) to render names; if any event is under-specified, that is a small, separately-tracked `engine-event-emission` enhancement, not part of this change's core.
- Verification: frontend unit tests (event fixtures → expected log lines) plus a manual/Playwright check that a real game produces a populated, scrolling log.
