## 1. Event -> text formatter

- [x] 1.1 Inventory the `GameEvent` variants the frontend receives (from `gameApi`/`engineDtos` and `engine-event-emission`) and decide the v1 in-scope set (Rust/PyO3 PascalCase set plus legacy lowercase aliases).
- [x] 1.2 Add `utils/gameEvents.ts` with `normalizeGameEvent(s)` so Rust/PyO3 PascalCase variants and legacy lowercase variants share canonical frontend event names; run create/action/step/surrender response events through it in `gameApi.ts`.
- [x] 1.3 Add `utils/gameLogFormat.ts` exporting a pure `formatEvent(event, ctx)` (and/or `formatEvents(events, state)`) that returns readable line(s), emitting `[CARD_ID:Name]` tokens so the existing clickable-card-name rendering applies.
- [x] 1.4 Resolve card/player names from the event payload first, then from current `player1`/`player2` zones; fall back to id/neutral text when unresolved. Omit player-irrelevant events.
- [x] 1.5 Unit tests: representative event fixtures -> expected lines; unknown/under-specified events are skipped without error; event normalization maps PascalCase and preserves legacy lowercase aliases.

## 1b. Desktop event plumbing

- [x] 1b.1 Implement Tauri `drain_events(game: &mut Game) -> Vec<GameEventDto>` by converting drained Rust `GameEvent`s into the existing DTO shape (same flattening convention as PyO3: common fields plus `meta`).
- [x] 1b.2 Add a Rust test/smoke assertion covering at least memory/play/digivolve/game-over event conversion and the empty-after-drain behavior.

## 2. Render the log from events

- [x] 2.1 Point `GameLog`/`GameLogDrawer` at a memoized selector over `store.events` (formatted via `gameLogFormat`) instead of `store.logs`; keep auto-scroll and the empty-state message.
- [x] 2.2 Preserve the clickable card-reference behavior (hover/preview wiring) for formatted lines.
- [x] 2.3 Ensure create-response events are appended so opening bot prelude events appear in the log.
- [x] 2.4 Verify the log includes both local and bot actions in arrival/sequence order.

## 3. Retire the dead textual log path

- [x] 3.1 Confirm nothing besides `GameLog` consumes `store.logs`; remove the `logs` store slice and `appendLogs` calls in `GamePage`/`gameStore`.
- [x] 3.2 Keep the browser/desktop `logs` response field present-but-empty for compatibility; remove frontend consumption of it and leave the binding `get_last_log()` stub documented.

## 4. Verification

- [ ] 4.1 Manual/Playwright check in a running bot game: the log populates and scrolls, shows both players' actions, and card names are clickable.
- [x] 4.2 Confirm log entries and board animations (digivolve/battle/security/effect where emitted) reflect the same normalized underlying events (no divergence).
- [x] 4.3 File any event found to lack data needed for a good log line as a separate `engine-event-emission` enhancement (do not block this change).
