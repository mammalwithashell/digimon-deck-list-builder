# Tasks — add-room-match-pvp

## 1. Guest-session hardening (frontend)

- [x] 1.1 `bootstrap/guest.ts`: return the offline sentinel in-memory only (never persist it); treat a cached literal `offline-guest-token` as absent and re-mint; update `guest.test.ts` for both behaviors
- [x] 1.2 `api/client.ts`: on 401 with no refresh token and a guest token (`is_guest` claim decoded client-side), clear cached guest keys, mint via `/auth/guest`, mirror into `access_token`, retry the request once (reuse the `_retry` guard); add unit tests for re-mint-once and surface-error-on-second-401
- [x] 1.3 `stores/authStore.ts` / `UserMenu`: ensure hydrate reflects a re-minted guest display name (Guest-XXXX, not the offline 'Guest') — hydrate re-reads cached keys updated by remint; the 401 interceptor also syncs the store mid-session

## 2. Lobby API redesign (server)

- [x] 2.1 `routers/lobby.py`: switch `_generate_join_code` to 5-digit numeric strings (leading zeros preserved, collision-retry unchanged); update any frontend display assumptions (code input length)
- [x] 2.2 Extend `PendingGame` with `joiner_user_id`, `joiner_display_name`, `joiner_deck`, `joiner_deck_raw`, `first_player ('1'|'random'|'2', default 'random')`, `started` flag
- [x] 2.3 Rework `POST /lobby/join/{code}`: reserve the seat (no deck, no game construction); 409 when seat taken, 400 when host self-joins, idempotent re-join for the seated user
- [x] 2.4 Rework `PUT /lobby/{id}/deck`: accept host or seated joiner, lock to the caller's seat; 403 for non-participants
- [x] 2.5 Add `PUT /lobby/{id}/first-player` (host-only, validates `1|random|2`) and `POST /lobby/{id}/leave` (joiner-only, clears seat + deck)
- [x] 2.6 Add `POST /lobby/{id}/start` (host-only): 409 unless both seats occupied and both decks locked; construct the game (task 3.2), set `joiner_user_id` on `GameSettings`, mark pending entry `started` (do NOT delete it — it is the seat map)
- [x] 2.7 Rework `GET /lobby/{id}/state`: require auth; return joiner presence/readiness, `first_player`, `started`, and `your_seat` for the caller; keep TTL pruning covering started entries
- [x] 2.8 Server tests for the lobby lifecycle (`code/tests/api/test_lobby_rooms.py`): create→join→decks→first-player→start happy path; seat exclusivity; premature start 409; leave/cancel; TTL expiry; numeric code format

## 3. Rust PvP runtime (server)

- [x] 3.1 Port `engine_py_legacy/engine/state_filter.py` (player + spectator filters) to `code/server/state_filter.py`; rule-14 regression test against a real `RustHeadlessGame.to_ui_json()` payload in `test_lobby_rooms.py` (existing `test_state_filter_modifiers.py` repointed)
- [x] 3.2 Lobby/matchmaking game construction: `create_pvp_game()` in lobby.py shared with matchmaking; `digimon_engine.parse_deck`; seed-parity direction pinned by `test_first_player_choice_maps_to_created_game`
- [x] 3.3 `routers/ws_games.py`: swapped runner type gate + API mapping; `action_descriptions` confirmed dead in the frontend (only an unused optional type field) and dropped
- [x] 3.4 `routers/ws_manager.py`: broadcast swap (per-player filtered state + mask only to the decision player, `your_player_id` added per recipient) and spectator broadcasts via the ported filter
- [x] 3.5 `routers/matchmaking.py`: legacy import removed; quick match constructs the Rust game directly at promote time (both decks already in tickets — no lobby handoff round-trip); tickets/messages carry `game_id` + `your_seat` instead of `join_code`
- [x] 3.6 Concede ordering verified at the WS layer: final state_update broadcast, then `game_over {winner_id, surrendered_by}` (covered by `test_ws_pvp_rust.py`)
- [x] 3.7 Guardrail test (`test_pvp_path_imports_no_legacy_engine`): AST-checks lobby/matchmaking/ws_games/ws_manager/state_filter import zero `engine_py_legacy`
- [x] 3.8 Integration test (`code/tests/api/test_ws_pvp_rust.py`): full two-guest room flow + live WS game on Rust — mulligans via action path, decision-routing rejection probe, 60-step play loop with rule-14 redaction asserted on every broadcast, concede→game_over; plus spectator redaction

## 4. Frontend room flow

- [x] 4.1 `api/lobbyApi.ts` + `features/play/playApi.ts`: deck-less `joinLobby(code)`, `setFirstPlayer`, `startLobby`/`startRoom`, `leaveLobby`/`leaveRoom`, `cancelRoom`; `LobbyState` extended with joiner fields, `first_player`, `your_seat`
- [x] 4.2 New `RoomChooserPage` at `/play/room` (Create Room / Join Room with 5-digit numeric code input); ModeSelectPage room mode routes there; legacy LobbyPage create/join repointed at the room screen
- [x] 4.3 `RoomLobbyPage`: 2s polling, seat-aware host/guest rendering, live opponent panel (name + deck-ready), joiner deck locking via the shared picker (server accepts per-seat deck PUT)
- [x] 4.4 Host controls: first-player selector (1 / RANDOM / 2) + START GAME enabled only when both decks ready; joiner sees readiness + waiting-for-host status; CANCEL/LEAVE per role
- [x] 4.5 Auto-navigation on `started` via `your_seat` for both clients (navigatedRef guard); room-gone 404 routes back to the chooser; MatchingPage/useMatchmaking/LobbyPage updated to the seat-based matchmaking contract (`game_id` + `your_seat`, no join_code)
- [x] 4.6 `playFlowStore`: `seat` field threaded through setQueue/clearLaunchState; typecheck + 102 unit tests green; desktop production build verified

## 5. End-to-end verification & deploy

- [x] 5.1 Two-context Playwright e2e (`code/frontend/e2e/room-match.spec.ts`) against a real local stack (uvicorn :8000 + vite :5174 with new ws-proxy support): two distinct guests complete create→code→join→decks→first-player→start→both auto-enter the live Rust game over WS; unknown-code error path covered. (Engine-level play-through + concede covered by `code/tests/api/test_ws_pvp_rust.py`.)
- [x] 5.2 Deployed to the DO droplet via `build-api-image.yml --deploy` (PR #623 + #624 merged to main). Production curl-verified: guest mint → create (5-digit code) → deck-less join → both decks → first_player=2 → start → both seats see `started: true` + `your_seat` — Rust game constructed live on the droplet.
- [ ] 5.3 Desktop build: `desktop-v0.3.0` tagged (version bump PR #625); CI building/publishing to the alpha updater channel. Poisoned-install recovery + a real two-client match to be confirmed on the installed build.
