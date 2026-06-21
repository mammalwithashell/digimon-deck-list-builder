## Context

The opponent's in-game name is the `playerLabels` (Zustand `gameStore`) display channel,
consumed by `components/board/GameBoard.tsx` (`playerLabels[2]`), `utils/gameLogFormat.ts`
(action log), and `components/game/ResultOverlay.tsx`. Nothing downstream needs to change —
only the value written into the labels.

Labels are set in exactly three places in `code/frontend/src/pages/GamePage.tsx`:

1. **WebSocket `onStateUpdate`** (PvP, vs-AI-online, spectator) — currently
   `{ [self]: 'You', [other]: isVsAiOnline ? 'AI' : 'Opponent' }`. **Called on every state
   update**, and only when `your_player_id != null` (so spectators, who have no seat, fall
   through and keep the store default `Player 1` / `Player 2`).
2. **HTTP hydrate path** (local/desktop games loaded by URL) — `{ 1: 'YOU', 2: 'GREEDY BOT'
   | 'OPPONENT' }`. Set once on load.
3. **HTTP create path** (GamePage's own start-game form) — prefers `result.player_labels`
   from the backend, else `{ 1: 'YOU', 2: 'GREEDY BOT' | 'OPPONENT' }`.

The backend builds `player_labels` with `'Agent'` for the non-human slot, but with the
frontend owning opponent naming for all in-game surfaces (below), those values are no longer
displayed.

This crosses only the frontend (hence a short design doc); no new dependency or schema.

## Goals / Non-Goals

**Goals:**
- Opponent shown under a Digimon alias for **all** in-game types (bot, PvP, vs-AI-online).
- Alias is **stable for the duration of a game** (no mid-game flicker, survives reload).
- Local player's own seat keeps `YOU`/`You`; spectators/replays alias both seats.
- One roster + one picker; frontend-only.

**Non-Goals:**
- True anonymization: withholding the real `display_name` on the wire (matchmaking
  `match_found` still carries it). This is display-only.
- Aliasing pre-game lobby/matchmaking screens (RoomLobbyPage, match-found).
- Any backend, engine, DSL, RL, tensor, action-space, or Tauri-wire change.
- Tying the alias to AI strength, policy, or deck archetype.

## Decisions

### Decision: Deterministic, game-seeded alias (not literally re-rolled)
The picker derives the alias from `game_id` (+ seat, for the two-alias spectator case) via the
existing FNV-1a hash already used by `starterIndexFromSeed` in `playApi.ts`:
`alias = ROSTER[fnv1a(`${gameId}:${seat}`) % ROSTER.length]`.
- *Why:* the WS `onStateUpdate` handler runs on **every** state tick; a literal `Math.random()`
  pick there would re-roll the opponent's name on each update — visible flicker. Seeding by a
  per-game key makes the alias a pure function of the game, so it is constant across ticks,
  reloads, and reconnects.
- *Bonus:* because both clients (and spectators) compute `f(gameId, seat)` from the same
  inputs, they all agree on the per-seat alias for free — without any server coordination.
- *Trade-off vs the originally-stated "completely random":* the alias is deterministic per
  game rather than re-rolled. It still varies game-to-game and is indistinguishable from random
  to a player. If true per-session re-rolling is ever wanted instead, the alternative is to pick
  randomly once and memoize per `gameId` (a `useRef`/`useMemo` keyed by game) — same stability,
  but loses reload/cross-client consistency. We chose game-seeding as the simpler, more robust
  default.

### Decision: Frontend owns opponent display naming for all in-game surfaces
All three label sites (plus the new spectator branch) set the opponent label to the alias
directly; the create path stops deferring the opponent slot to `result.player_labels`.
- *Consequence:* no backend change is needed. `games.py` / `debug_games.py` keep emitting
  `'Agent'`, which is simply never displayed. This reverses the backend-parity work from the
  earlier (bot-only) iteration of this proposal — it is now redundant.
- *Alternative considered:* alias on the backend and have the client trust it. Rejected — the
  PvP/WS and spectator labels are computed client-side anyway, and the username-vs-alias choice
  is fundamentally a viewer-relative display decision (self = `YOU`), which the client is best
  placed to make.

### Decision: Self stays `YOU`; spectators/replays alias both seats
Only the seat that is *not* the local viewer is aliased. When `your_player_id` is null
(spectator/replay), there is no "self", so both seats are aliased via the seeded picker. This
adds a branch where today spectator labels are left at the default `Player 1`/`Player 2`.

### Decision: Apply at label-set time, reading the alias from the shared util
The picker lives in one util (e.g. `features/play/botNames.ts`) exporting `BOT_NAMES` and
`pickAlias(seed: string)`. Every label site imports it. No per-call-site name lists.

## Risks / Trade-offs

- [Mid-game name flicker] → Mitigated by game-seeding the alias (constant per game); never call
  the random picker inside `onStateUpdate`.
- [Not true anonymity — username still on the wire] → Accepted and documented (display-only,
  "anon for now"). A later change can withhold/redact `display_name` at the matchmaking layer
  if real confidentiality is wanted.
- [Pre-game screens still show real names] → Out of scope; noted so the gap is intentional.
- [Tests asserting literal `'GREEDY BOT'` / `'AI'` / `'Opponent'` labels] → Audit and relax to
  "label is a roster member / not a placeholder".
- [Two clients could disagree on aliases] → Avoided by deterministic game-seeding (they agree).
- [Trademark/IP] → Names used as flavor text in a non-commercial fan simulator, consistent with
  existing use of card names/archetypes.

## Migration Plan

Pure additive cosmetic change; no data migration, no deployment ordering. Rollback = revert the
label-site edits to their prior literal strings.

## Open Questions

- None blocking. (Roster contents, randomness model, scope, and self-label were settled during
  exploration; the game-seeding refinement is flagged for the user to veto if they truly want
  per-session re-rolling.)
