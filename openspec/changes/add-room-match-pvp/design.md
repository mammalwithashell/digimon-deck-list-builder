# Design — add-room-match-pvp

## Context

Verified current state (explored against code and the live DigitalOcean server):

- **Guest auth works server-side.** `POST /auth/guest` mints a year-long anonymous JWT backed by a real DB `User` row (`is_guest=True`); `POST /lobby/create` with that bearer returns a join code (curl-verified HTTP 200). CORS preflight for the Tauri webview origin (`http://tauri.localhost`) passes.
- **The desktop client is the broken half.** `code/frontend/src/bootstrap/guest.ts` persists `OFFLINE_GUEST_SESSION` (token literal `'offline-guest-token'`, display name `'Guest'`) to localStorage if the *first ever* mint attempt fails for any reason, and `ensureGuestSession()` reuses any cached token forever without revalidation. The axios 401 interceptor only knows how to refresh (guests have no refresh token), so every authed call fails permanently → "UNABLE TO CREATE ROOM".
- **The lobby API is one-phase.** `POST /lobby/join/{code}` requires the joiner's deck in the request and immediately constructs the game; `joiner_deck_ready` is hardcoded `false`; `PUT /lobby/{id}/deck` is host-only; there is no first-player choice and no explicit start. The host UI (`RoomLobbyPage.tsx`) never polls room state and its ENTER GAME button can navigate to a game that does not exist yet.
- **PvP runs the legacy engine.** `lobby.py`, `matchmaking.py`, `ws_games.py`, `ws_manager.py` import `engine_py_legacy` (`InteractiveGame`, `state_filter`, `parse_deck`, `PlayerType`) — rule-22 violations. Meanwhile `routers/games.py` has already migrated to `digimon_engine.RustHeadlessGame` and is the proven pattern.
- **`RustHeadlessGame` already exposes the needed surface**: `step`, `get_action_mask`, `to_ui_json` (includes `pendingSelection`; mirrors the legacy JSON shape), `get_last_log`, `get_events_since_last_step`, `current_player_id` (the *decision* player, correct for mid-turn selections), `is_game_over`, `winner_id`, `concede(pid)`, seeded construction. Mulligan keep/redraw are actions 0/1 through the normal mask/step path. `Game::new` selects first player from `seed % 2` (documented, relied on by BO3 training, rule 26).
- **`state_filter` is engine-independent.** `engine_py_legacy/engine/state_filter.py` is pure dict surgery (redacts `handIds`/`handCards`/`securityIds`, preserves counts) with no engine imports; the Rust `to_ui_json` intentionally matches the dict shape it filters.
- A parked change (`excise-legacy-engine-from-hosted-api`, DEFERRED) sketches the full server-wide excision. This change implements only its PvP-path subset under an explicit user decision; that proposal's "new interactive PyO3 runner" assumption is obsoleted by `games.py`'s migration.

Constraints: no user accounts (guest JWTs only); single-instance hosted API (in-memory lobby + `active_games` are acceptable); desktop frontend talks to `VITE_API_URL` over HTTPS/WSS; rule 14 (redact opponent `handIds` *and* `handCards`), rule 20 (Python 1/2 player-ID convention at the binding boundary), rule 22 (no `engine_py_legacy` imports in production code).

## Goals / Non-Goals

**Goals:**

- Two desktop clients complete the full DCGO-style room flow: create → share 5-digit code → join → both lock decks → host picks first player → host starts → both play to completion over WebSocket.
- The desktop app recovers from a poisoned/stale guest token without user intervention.
- The PvP path (`lobby`, `matchmaking`, `ws_games`, `ws_manager`) imports zero `engine_py_legacy` symbols and runs games on `RustHeadlessGame`.
- Opponent hidden state is never leaked to the other player or to spectators (rule 14 contract preserved over the Rust state).

**Non-Goals:**

- Replays, recordings, simulations, deck DB routes, admin AI — they keep their legacy imports (remain in the parked excision change).
- Lobby push (WebSocket/SSE) — polling is sufficient at alpha scale.
- Public-lobby browser UX changes, spectator UX changes, BO3 rooms, reconnect-to-finished-room recovery beyond what exists.
- Engine or PyO3-binding changes (none are expected to be needed).

## Decisions

### D1. Guest tokens: never persist the offline sentinel; re-mint on guest 401

`ensureGuestSession()` returns the offline sentinel **in-memory only** (per launch) when the API is unreachable; nothing is written to localStorage on the failure path, so the next launch retries a real mint. For 401 recovery: the axios interceptor, on a 401 with no refresh token, checks whether the stored token is a guest token (decode the JWT payload client-side — `is_guest: true`); if so it clears the cached guest keys, mints a fresh guest via `/auth/guest`, mirrors it into `access_token`, and retries the request once. Guests own nothing server-side (decks are client-local on desktop), so a new identity on auth failure is free by design — this deliberately reverses guest.ts's old "never silently re-mint" policy, which is what bricked the app. Alternative considered: validating the cached token at boot with a `/auth/me` ping — rejected as the *primary* fix because it adds a launch-path network dependency and still doesn't cover mid-session invalidation; 401-triggered re-mint covers both. One-time migration: treat the exact literal `'offline-guest-token'` in localStorage as absent.

### D2. Room codes: 5-digit numeric, collision-checked, TTL-pruned

`_generate_join_code` produces 5 digits (`00000`–`99999`, leading zeros kept as a string). 100k codespace is ample for an alpha with a 30-minute TTL and the existing per-IP guest rate limit; collisions are retried against `code_to_game` as today. Easier to read aloud than alphanumeric (the user's decision, matching DCGO).

### D3. Two-phase lobby with an explicit start gate

`PendingGame` gains `joiner_user_id`, `joiner_display_name`, `joiner_deck`, `joiner_deck_raw`, `first_player ('1'|'random'|'2', default 'random')`. New lifecycle:

```
create(host) ──► waiting ──join(code)──► seated ──both decks──► ready ──start(host)──► started
                   ▲                      │ leave(joiner)                │ RustHeadlessGame in
                   └──────────────────────┘                              │ active_games; pending
                 cancel(host) deletes room at any pre-start point        ▼ kept as seat map
```

- `POST /lobby/join/{code}`: reserves the seat (409 if taken), **no deck required, game not constructed**. Idempotent for the same user (re-join returns the same room).
- `PUT /lobby/{id}/deck`: host *or* seated joiner; sets that seat's deck.
- `PUT /lobby/{id}/first-player` (host-only): `1 | random | 2`.
- `POST /lobby/{id}/leave` (joiner): vacates the seat and clears the joiner deck.
- `POST /lobby/{id}/start` (host-only): 409 unless both seats occupied and both decks locked; constructs the game.
- `GET /lobby/{id}/state` (authed — see D5): full readiness picture plus, when started, the caller's seat.

Breaking the old join-with-deck contract is acceptable: the only consumers are our own frontend pages, which this change updates. Alternative considered: keeping auto-start-on-join and bolting presence on top — rejected because it cannot express the DCGO room (joiner picks a deck *inside* the room, host controls the start).

### D4. Start = seed parity; pending game survives as the seat map

On start: pick a random `u64` seed, then force its parity to match the host's first-player choice (`Game::new` first player = `seed % 2`; random choice = leave the seed alone). Construct `RustHeadlessGame(host_deck, joiner_deck, seed=seed)` into `active_games[game_id]`, set `joiner_user_id` on the manager's `GameSettings` (the existing WS seat-validation mechanism), and mark the pending game `started` rather than deleting it (today's code deletes it, which is why a host polling `/state` loses the room the instant the game begins). The started pending entry holds the user→seat map for `/state` responses and is pruned by the existing TTL sweep. The exact parity→player-1 mapping is asserted by a test rather than assumed (the BO3 wrapper already relies on this trick; the test pins the direction).

### D5. State endpoint is authenticated and seat-aware

`GET /lobby/{id}/state` switches from anonymous to authenticated (guest JWTs pass — they are normal bearer tokens), so the response can carry `your_seat: 1|2|null` and per-seat readiness without leaking deck contents. Response shape (additive otherwise): `{game_id, join_code, host_display_name, joiner_display_name, host_deck_ready, joiner_deck_ready, first_player, started, your_seat, allow_spectators, spectator_mode}`. Both clients poll at ~2s; on `started: true` each navigates to `/game/{game_id}?mode=pvp&player={your_seat}`. Alternative considered: returning seat assignment only from `/start` — rejected because the joiner never calls `/start`; polling must be sufficient for both sides.

### D6. PvP runtime swaps to `RustHeadlessGame`; `games.py` is the pattern

`ws_games.py`/`ws_manager.py`/`matchmaking.py`/`lobby.py` replace `InteractiveGame` with `RustHeadlessGame`:

- **Turn/decision routing**: validate `runner.current_player_id == player_id` (the binding's decision-player semantics make this correct for mid-turn selections owned by the defender); send the action mask only to the decision player. Mulligan flows through actions 0/1 — no special message type.
- **API mapping**: `surrender(pid)` → `concede(pid)`; `get_last_events()/clear_events()` → `get_events_since_last_step()` (drains once — call once per step and fan the buffer out to all recipients); `game.winner.player_id` → `winner_id`; `runner.game.to_ui_json()` → `runner.to_ui_json()`.
- **`action_descriptions`** (legacy WS field): verified absent from the Rust HTTP path that the frontend already consumes for bot games; confirm the WS consumer tolerates its absence and drop it.
- **Deck parsing**: `digimon_engine.parse_deck` (as `games.py` does); the `PlayerType` import disappears with `InteractiveGame`.
- **Concurrency**: PyO3 calls are synchronous and GIL-holding; FastAPI handlers on one event loop serialize access between the two players' sockets — same model as the legacy runner, no locking added.

Type gates change from `isinstance(runner, InteractiveGame)` to `isinstance(runner, RustHeadlessGame)`; `games.py`-created games and lobby-created games now share one runner type, which also lets the existing reconnect/slot logic in `ws_games.py` work unchanged.

### D7. State redaction ports to `code/server/state_filter.py` verbatim

The legacy module is dependency-free dict surgery whose input shape the Rust `to_ui_json` deliberately mirrors. Copy it (player + spectator filters) into `code/server/`, update imports in `ws_games.py`/`ws_manager.py`, and add a regression test asserting opponent `handIds`, `handCards`, and both players' `securityIds` are stripped from a real Rust `to_ui_json()` payload (rule 14 over the new state source). Alternative considered: a Rust-native `to_ui_json_for_player(pid)` — better long-term home, but it adds a bindings change this flow doesn't need; the parked excision change can absorb that later without touching the contract.

### D8. Frontend flow: chooser → join page → seat-aware room page

- Room-match entry presents **Create Room / Join Room** (DCGO screenshot parity). Join Room is a new page with a 5-digit code input that calls the deck-less `joinLobby(code)` and routes to `/play/room/{game_id}`.
- `RoomLobbyPage` becomes seat-aware (host vs joiner from `your_seat`) and polls `getRoomState` every ~2s: live opponent panel (name + deck-ready), first-player selector rendered for the host (`1 / RANDOM / 2`), START button host-only and enabled only when the state says both decks are ready, joiner sees readiness instead of a start control. On `started`, both sides navigate using `your_seat` — the manual ENTER GAME footgun (navigating to a not-yet-created game) is removed.
- `lobbyApi.ts`/`playApi.ts` gain `setFirstPlayer`, `startRoom`, `leaveRoom` and the deck-less join; `playFlowStore` carries the seat.

## Risks / Trade-offs

- **[Rust runner behaves differently from legacy in live PvP]** — the engine is the better-tested of the two (it's the RL/desktop engine), and bot games already run it on this server via `games.py`. Mitigation: an integration test driving a full two-human game through the lobby + WS path (create→join→decks→start→mulligans→a few turns→concede); any genuine rules divergence routes to the standard engine-gap process.
- **[Decision-player routing breaks on some interrupt timing]** — if any pending-selection state misattributes `current_player_id`, a player gets "Not your turn" on a legal prompt. Mitigation: the same semantics already drive desktop play and the RL mask; WS test covers a defender-side selection (e.g. blocker prompt).
- **[Concede event ordering vs rule 16]** — the legacy contract emits `surrender` before `game_over`. Verify `concede()`'s event order in `get_events_since_last_step()`; if the Rust path lacks a surrender-equivalent event, synthesize the WS-level `game_over {surrendered_by}` message as today (the WS message, not the engine event, is what the frontend consumes).
- **[Guest re-mint loops]** — a server that 401s every guest token (e.g. rotated `SECRET_KEY`) would cause mint-per-request churn. Mitigation: re-mint at most once per request via the existing `_retry` flag, and the `/auth/guest` per-IP rate limit (10/min) caps the blast radius.
- **[Started-room state lingering in memory]** — keeping started pending entries for seat lookup adds a small leak surface. Mitigation: existing TTL sweep prunes them; `active_games` cleanup on game-over is unchanged.
- **[5-digit codespace collisions under load]** — negligible at alpha scale (TTL'd rooms, retry-on-collision); revisit only if public-lobby volume grows.
- **[Two clients polling at 2s]** — trivial load; accepted in lieu of a lobby WS (explicit non-goal).

## Migration Plan

1. Ship the frontend guest fix and the server lobby/WS changes together in one deploy (the desktop app talks to the live API, so server first, then a desktop release build; the old desktop build's room flow is already broken, so there is no working old-client contract to preserve).
2. Deploy order: hosted API (DO droplet) → verify with two curl/Playwright guests end-to-end → cut a desktop build.
3. Existing desktop installs recover automatically: the poisoned `'offline-guest-token'` is migrated away client-side on first launch of the new build.
4. Rollback: server-side revert restores the legacy lobby (old desktop builds were equally broken either way); no DB migrations to unwind.

## Open Questions

- Does the WS frontend consumer (`useWebSocketGame`/`GamePage`) need `logs` in the `state_update` payload shape exactly as `broadcast_state` sends today? (Implementation will mirror the existing message shape; flagged only as a verify-during-build.)
- Spectator mode over Rust games: ported filter keeps the contract, but spectator UX is untested in this change (explicit non-goal beyond not breaking the endpoint).
