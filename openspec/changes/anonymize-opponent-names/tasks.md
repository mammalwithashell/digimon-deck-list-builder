## 1. Frontend roster + seeded picker

- [x] 1.1 Create `code/frontend/src/features/play/botNames.ts` exporting `BOT_NAMES` (the approved roster, all tiers incl. Kunlun) and `pickAlias(seed: string): string` that returns `BOT_NAMES[fnv1a(seed) % BOT_NAMES.length]`. Reuse / extract the existing FNV-1a hash from `playApi.ts` (`starterIndexFromSeed`) so there is one hash impl. — extracted to `features/play/hash.ts`, `playApi.ts` now reuses it.
- [x] 1.2 Unit test (`botNames.test.ts`): `pickAlias` always returns a member of `BOT_NAMES`; same seed → same name (stable); different seeds spread across the roster; `BOT_NAMES` is non-empty with no duplicates.

## 2. Wire the opponent alias into every in-game label site (GamePage.tsx)

- [x] 2.1 WebSocket `onStateUpdate`: opponent label seeded by `game_id` (not random) so it doesn't flicker across state ticks. Extracted to `liveGameLabels(gameId, yourPlayerId)`.
- [x] 2.2 Spectator/replay branch: `your_player_id == null` → both seats aliased (handled inside `liveGameLabels`).
- [x] 2.3 HTTP hydrate path: replaced `2: 'GREEDY BOT' | 'OPPONENT'` with `pickAlias(`${urlGameId}:2`)`; kept `1: 'YOU'`. Removed the now-unused `opponentMode` destructure.
- [x] 2.4 HTTP create path: opponent seat set to `pickAlias(`${result.game_id}:2`)`; removed the `result.player_labels` deferral (client owns opponent naming). Kept `1: 'YOU'`.
- [x] 2.5 Grepped the frontend: only remaining `GREEDY BOT`/`'AI'`/`'Opponent'` literals are out-of-scope pre-game lobby (`RoomLobbyPage`) and the `ModeSelectPage` window title — not in-game player labels.

## 3. Test/audit fallout

- [x] 3.1 Grepped tests: no production test asserts the opponent-label literals. The `playerLabels: { 2: 'Bot' }` fixtures in `gameLogFormat.test.ts` / `ResultOverlay.test.tsx` are consumer-side inputs (unaffected); `aiStarter.test.ts` asserts `player_kinds`, not labels.
- [x] 3.2 Added `liveGameLabels` tests in `botNames.test.ts`: seated → self `'You'` + opponent alias; spectator → both seats aliased (no `Player 1/2`); stable across repeated calls; never emits a placeholder label.

## 4. Verify

- [x] 4.1 Frontend unit tests: targeted suite (botNames, aiStarter, playFlowStore, gameLogFormat, ResultOverlay) = 31/31 green. Full suite = 232 passed / 1 failed; the single failure (`CardOverlay.test.tsx`, a `[Your Turn]` timing-tag rendering mismatch) is **pre-existing and unrelated** — that file is untouched committed code (last changed 10 days ago) and is independent of player-label aliasing.
- [x] 4.2 Typecheck: `tsc -b` exits 0 (clean).
- [ ] 4.3 Manual (needs interactive launch): desktop vs-AI game (run-desktop) — opponent name tag shows a roster alias (not `GREEDY BOT`), action log + result overlay use the same alias, `YOU` unchanged, and the name does not change across turns.
- [ ] 4.4 Manual (needs interactive browser-dev PvP): a human-vs-human game shows the opponent under an alias (not their account name) on both clients, and both clients agree on the per-seat aliases.
