## 1. Event → text formatter

- [ ] 1.1 Inventory the `GameEvent` variants the frontend receives (from `gameApi`/`engineDtos` and `engine-event-emission`) and decide the v1 in-scope set (play, digivolve, attack, security check, effect activate/fizzle, memory/turn/phase, deletion, game over).
- [ ] 1.2 Add `utils/gameLogFormat.ts` exporting a pure `formatEvent(event, ctx)` (and/or `formatEvents(events, state)`) that returns readable line(s), emitting `[CARD_ID:Name]` tokens so the existing clickable-card-name rendering applies.
- [ ] 1.3 Resolve card/player names from the event payload first, then from current `player1`/`player2` zones; fall back to id/neutral text when unresolved. Omit player-irrelevant events.
- [ ] 1.4 Unit tests: representative event fixtures → expected lines; unknown/under-specified events are skipped without error.

## 2. Render the log from events

- [ ] 2.1 Point `GameLog`/`GameLogDrawer` at a memoized selector over `store.events` (formatted via `gameLogFormat`) instead of `store.logs`; keep auto-scroll and the empty-state message.
- [ ] 2.2 Preserve the clickable card-reference behavior (hover/preview wiring) for formatted lines.
- [ ] 2.3 Verify the log includes both local and bot actions in arrival/sequence order.

## 3. Retire the dead textual log path

- [ ] 3.1 Confirm nothing besides `GameLog` consumes `store.logs`; remove the `logs` store slice and `appendLogs` calls in `GamePage`/`gameStore`.
- [ ] 3.2 Decide and apply the server `logs` response-field disposition (keep present-but-empty vs remove) after checking consumers; leave the binding `get_last_log()` stub documented.

## 4. Verification

- [ ] 4.1 Manual/Playwright check in a running bot game: the log populates and scrolls, shows both players' actions, and card names are clickable.
- [ ] 4.2 Confirm log entries and board animations (digivolve/battle) reflect the same underlying events (no divergence).
- [ ] 4.3 File any event found to lack data needed for a good log line as a separate `engine-event-emission` enhancement (do not block this change).
