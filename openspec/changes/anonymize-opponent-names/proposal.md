## Why

Today the opponent is presented inconsistently and un-anonymously: bot games show the
hardcoded placeholder `GREEDY BOT` / `AI`, while human-vs-human games surface the
opponent's real account `display_name`. We want a single, consistent presentation across
**all** game types: the opponent appears under a randomized Digimon-franchise alias, so
single-player feels like a real named match and multiplayer is anonymous-by-default for now.
This is purely cosmetic — a display concern, not a gameplay or identity-security change.

## What Changes

- Introduce one shared frontend roster of Digimon-character names plus an alias picker that
  is **stable per game** (seeded from `game_id`) — so the opponent's name doesn't change
  mid-game on WebSocket state updates, survives reload, and (as a side effect) is the same on
  both clients and for spectators.
- Apply the alias to the **opponent** seat in every in-game label site:
  - WebSocket path (PvP and vs-AI-online) — replaces `'AI'` / `'Opponent'`.
  - Local/desktop HTTP hydrate path — replaces `'GREEDY BOT'` / `'OPPONENT'`.
  - HTTP create path — replaces the `'GREEDY BOT'` fallback and stops deferring the opponent
    label to the backend.
- Add a **spectator/replay** branch (no local seat) that aliases **both** seats, where today
  labels are left at the default `Player 1` / `Player 2`.
- The local player's own seat keeps `YOU` / `You` (unchanged). In PvP, only what the *other*
  player is shown is aliased.
- **No backend changes.** The frontend now owns opponent display naming for all in-game
  surfaces, so the Python label generators (`games.py`, `debug_games.py`) are left as-is and
  their values are simply not displayed.
- Display-only / cosmetic: no engine, DSL, RL, tensor, action-space, or gameplay-logic
  change. The alias flows solely through the existing `playerLabels` channel (board name tag,
  action log, result overlay).

## Capabilities

### New Capabilities
- `opponent-display-aliasing`: Defines that, across all in-game surfaces and game types, the
  player opposing the local viewer is presented under a randomized Digimon-character alias
  that is stable for the duration of a game, while the local player's own seat label is
  preserved; spectators/replays see both seats aliased. Cosmetic display only — not an
  identity-confidentiality guarantee.

### Modified Capabilities
<!-- None: no existing spec governs opponent display naming. -->

## Impact

- **Frontend only** (`code/frontend/src/`):
  - New util (e.g. `features/play/botNames.ts`): the roster + a `game_id`-seeded
    `pickAlias(seed)` (reusing the existing FNV-1a hash from `playApi.ts`).
  - `pages/GamePage.tsx`: three `setPlayerLabels` sites updated (WS `onStateUpdate`, HTTP
    hydrate, HTTP create) + a new spectator branch that aliases both seats.
  - Rendering is unchanged — `components/board/GameBoard.tsx`, `utils/gameLogFormat.ts`, and
    `components/game/ResultOverlay.tsx` already consume `playerLabels`.
- **No change** to the backend, the Rust engine, the Tauri wire, DSL/cards, or RL/training.
- **Out of scope (documented non-goals):** withholding the real `display_name` on the wire
  (it is still transmitted in the matchmaking `match_found` payload — this is display-only,
  not true anonymization); aliasing the *pre-game* lobby/matchmaking screens (RoomLobbyPage,
  match-found), which read separate identity fields.
- Low risk: a cosmetic display string. PvP gameplay and the human's own label are unaffected.
