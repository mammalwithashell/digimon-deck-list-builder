# add-room-match-pvp

## Why

Room-code PvP between two desktop clients is currently broken end-to-end: the desktop app gets stuck on a poisoned offline guest token ("UNABLE TO CREATE ROOM"), the lobby API has no guest-side flow (joining requires a deck up front and instantly starts the game, so there is no DCGO-style room where both players pick decks and ready up), and the games it would create run on the sunset Python legacy engine. The hosted API and guest auth are verified working — the gaps are the client token lifecycle, the lobby's join/ready/start shape, and the engine backing online play.

This change makes the DCGO room flow (host creates room → shares numeric code → guest joins → both lock decks → host picks first player → host starts → both enter the game) work between desktop clients with **no user accounts** (guest JWTs only), and moves the entire online-PvP runtime to the **Rust engine** — the user's explicit decision that nothing should run on `engine_py_legacy`.

## What Changes

- **Guest-session hardening (frontend)**: never persist the offline guest sentinel to localStorage (in-memory per launch only); recover from 401s on guest tokens by re-minting a fresh guest identity instead of failing forever. Fixes the current "UNABLE TO CREATE ROOM".
- **Lobby API redesign** (`code/server/routers/lobby.py`):
  - Room codes become **5-digit numeric** (DCGO-style), replacing 6-char alphanumeric.
  - **BREAKING** `POST /lobby/join/{code}` reserves a seat without a deck and without starting the game (today it requires the deck and immediately constructs the game).
  - Joiner can lock/replace their deck in the room; lobby state reports real joiner presence and readiness (today `joiner_deck_ready` is hardcoded `false`).
  - Host selects first player: `1 | random | 2`.
  - New host-only `POST /lobby/{id}/start` constructs the game once both decks are locked; lobby state carries per-user seat assignment so both clients navigate into the game.
  - Joiner can leave a room; host can cancel; both clients poll lobby state (~2s, alpha scale — no lobby WebSocket).
- **Rust engine for online play**: `lobby.py`, `matchmaking.py`, `ws_games.py`, `ws_manager.py` swap `engine_py_legacy` `InteractiveGame` → `digimon_engine.RustHeadlessGame` (following the already-migrated `games.py` pattern). Deck parsing in the PvP path moves to the Rust `parse_deck`. Per-player/spectator state redaction is ported off `engine_py_legacy.engine.state_filter` to a server-owned module with the same contract (never leak opponent `handIds`/`handCards`; rule 14). First-player choice is implemented via the documented seed-parity behavior of `Game::new` — no engine changes required.
- **Frontend room flow** (`code/frontend/src`): Create-or-Join chooser in the room-match flow; a Join Room code-entry page (5-digit); `RoomLobbyPage` gains state polling, a live opponent panel, host first-player selector, and a host-only start button; a joiner variant of the room screen; both sides auto-navigate to `/game/{id}?mode=pvp&player=N` when the room starts.

**Relationship to the parked `excise-legacy-engine-from-hosted-api` change**: this change implements the PvP-path subset of its phases 3–4 (state redaction + live game runtime for `lobby`/`matchmaking`/`ws_*`), under the user's explicit prioritization of Rust-backed online play. Replays, recordings, deck-rule routes outside the PvP path, and admin-AI script promotion remain deferred there. The parked change's assumption that a *new* interactive PyO3 runner is needed is obsolete — `RustHeadlessGame` already exposes the required surface (proven by `games.py`).

## Capabilities

### New Capabilities

- `guest-session`: anonymous guest identity lifecycle on the client — minting via `/auth/guest`, caching, offline behavior, and 401 recovery — with no user accounts anywhere in the PvP flow.
- `room-match-lobby`: the two-phase room lifecycle — create with numeric code, seat reservation on join, per-seat deck locking and readiness, host first-player selection, host-gated start, leave/cancel semantics, and polled room state with seat assignment.
- `pvp-game-runtime`: online PvP games execute on the Rust engine (`RustHeadlessGame`) over the existing WebSocket transport, with per-player and spectator state redaction, decision-player action routing (including mid-turn selections and mulligan), concede, and game-over reporting — zero `engine_py_legacy` imports in the PvP path.

### Modified Capabilities

(none — no existing specs cover the lobby, guest auth, or the PvP runtime)

## Impact

- **Frontend**: `src/bootstrap/guest.ts`, `src/stores/authStore.ts`, `src/api/lobbyApi.ts`, `src/features/play/playApi.ts` + `playFlowStore.ts`, `src/pages/RoomLobbyPage.tsx`, new Join Room page + route, play-flow chooser.
- **Server**: `routers/lobby.py` (API redesign + Rust runner), `routers/matchmaking.py`, `routers/ws_games.py`, `routers/ws_manager.py`, new `server/state_filter.py`; removes the PvP path's `engine_py_legacy` imports (rule 22).
- **No engine or bindings changes expected**: `RustHeadlessGame` already exposes step/mask/`to_ui_json`/events/concede/`current_player_id`; first player rides on seed parity.
- **Wire contracts**: WS `state_update` payload shape stays frontend-compatible; legacy-only `action_descriptions` is verified dead and dropped. Lobby REST responses gain fields (joiner presence, first player, seat assignment) and change join semantics (**BREAKING** for any client calling `/lobby/join` with a deck — only our own frontend does).
- **Deployment**: hosted API on DigitalOcean must build/ship the `digimon-engine-py` wheel (already required by the migrated `games.py`); no DB migrations (lobby state stays in-memory, single-instance).
- **Out of scope**: replays/recordings/simulations legacy usage, admin AI, lobby WebSocket push, spectator UX changes, BO3 rooms.
