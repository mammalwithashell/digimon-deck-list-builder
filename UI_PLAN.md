# UI & API Plan — Digimon Game Simulator

> Updated 2026-02-15. Reflects engine state after merge of main: 981-float tensor,
> 3,651-card database, DNA Digivolution, 15 keyword mechanics, 10 selection phases,
> revealed cards zone, linked option cards, and pilot_training.py agent infrastructure.

## Overview

Four main surfaces:

1. **Game UI** — Play interactive games (Human vs Agent, Human vs Human)
2. **Lobby** — Matchmaking, room codes, game creation
3. **Replay Viewer** — Play back recorded Agent vs Agent games
4. **Admin Dashboard** — Manage agents, launch training runs, view metrics

Tech stack: **React 19 + TypeScript + Vite**, with **Zustand** for state management and **WebSocket** for real-time game communication. The existing **FastAPI** backend is extended with new endpoints.

---

## 1. Game UI — Interactive Play

### 1.1 Board Layout

Inspired by the [WE-Kaito simulator](https://github.com/WE-Kaito/digimon-tcg-simulator), the board is a single-screen layout with mirrored player halves. All zones from the Digimon TCG are represented:

```
┌──────────────────────────────────────────────────────────────┐
│  OPPONENT AREA (top half, cards inverted)                    │
│  ┌─────┐ ┌─────────────────────────────────┐ ┌────┐ ┌────┐ │
│  │Egg  │ │  Battle Area (12 slots)         │ │Deck│ │Sec │ │
│  │Deck │ │  [Perm][Perm][Perm]...          │ │    │ │ury │ │
│  └─────┘ └─────────────────────────────────┘ └────┘ └────┘ │
│  ┌─────┐                                            ┌────┐ │
│  │Breed│                                            │Trsh│ │
│  └─────┘                                            └────┘ │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Revealed Cards Zone (face-up, during reveal effects) │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │            MEMORY GAUGE  [-10 ... 0 ... +10]         │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────┐                                            ┌────┐ │
│  │Breed│                                            │Trsh│ │
│  └─────┘                                            └────┘ │
│  ┌─────┐ ┌─────────────────────────────────┐ ┌────┐ ┌────┐ │
│  │Egg  │ │  Battle Area (12 slots)         │ │Deck│ │Sec │ │
│  │Deck │ │  [Perm][Perm][Perm]...          │ │    │ │ury │ │
│  └─────┘ └─────────────────────────────────┘ └────┘ └────┘ │
│                                                              │
│  YOUR AREA (bottom half)                                     │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Hand: [Card][Card][Card][Card]...                   │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────┐  ┌──────┐  ┌──────────────────────────────┐  │
│  │Phase Ind.│  │ Pass │  │ Game Log (scrollable)         │  │
│  └──────────┘  └──────┘  └──────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
│  Card Detail Sidebar (350px right panel, shows hovered card) │
```

### 1.2 Core UI Components

| Component | Description | Interactions |
|-----------|-------------|--------------|
| `GameBoard` | Root layout, CSS grid, two mirrored halves | — |
| `PlayerHalf` | One player's full zone set | — |
| `BattleArea` | 12 `PermanentSlot` components in a row, scrollable if >8 | Drop target for play/digivolve |
| `PermanentSlot` | Top card art, DP badge, level badge, suspend tilt (30deg), keyword badges, linked cards, digivolution stack depth | Click to select attacker/target, hover for detail |
| `KeywordBadges` | Small tags on permanents: Rush, Blocker, Jamming, Piercing, Retaliation, Blitz, Reboot, Collision, Evade, Armor Purge, Barrier, Security Attack +/-X | Display only |
| `LinkedCards` | Sideways mini-cards rendered next to the permanent (for [TS] option cards) | Click to view |
| `HandZone` | Horizontally fanned cards, dynamic spacing, compresses beyond 7 | Click card to see valid actions (play, digivolve, DNA targets) |
| `SecurityStack` | Face-down pile with count badge | Hover shows count, click to browse (own only) |
| `DeckPile` | Face-down pile with count badge | — |
| `EggDeck` | Digitama pile with count | Click for hatch action |
| `BreedingArea` | Single permanent slot | Click for move-to-battle action |
| `TrashPile` | Count badge, click to browse | Modal dialog listing all cards |
| `RevealedCardsZone` | Temporary row of face-up cards between the halves | Click to select during `SelectReveal` |
| `MemoryGauge` | 21-segment horizontal bar, color-coded (blue positive, red negative) | Display only (updated by server) |
| `PhaseIndicator` | Shows current phase name + turn number (15 phases, see §1.7) | Display only |
| `CardDetail` | Right sidebar, shows full card image + text + inherited effects when hovering | — |
| `GameLog` | Scrollable text panel showing VerboseLogger output | Auto-scrolls to bottom |
| `ActionBar` | Contextual buttons based on game state and current phase | Pass, Hatch, Move from Breeding, confirm target, decline optional |

### 1.3 Interaction Flow

The game uses a **click-to-act** model (not drag-and-drop). This is simpler to implement and works well on mobile. Drag-and-drop can be added later.

**Playing a card from hand:**
1. Player clicks a card in hand
2. UI highlights the card; action bar shows "Play" button (if action mask allows it)
3. Player clicks "Play" → sends action to backend
4. Backend returns new state + logs

**Attacking:**
1. Player clicks an unsuspended permanent in their battle area → it highlights as "attacker"
2. Valid targets light up (opponent permanents + security icon)
3. Player clicks a target → sends attack action (100 + attacker*15 + target)
4. **BlockTiming**: If opponent has blockers, UI shows "Block?" prompt to opponent with valid blockers highlighted, plus "Decline" button
5. **CounterTiming**: If opponent can blast digivolve, UI shows blast digivolve options, plus "Decline" button
6. Backend resolves combat, returns new state

**Digivolving:**
1. Player clicks a card in hand that can digivolve
2. Valid digivolution targets on the field highlight
3. Player clicks a target permanent → sends digivolve action (400 + hand*15 + field)

**DNA Digivolving (new):**
1. Player clicks a hand card with DNA digivolve capability
2. Action bar shows "DNA Digivolve" button → sends action (63 + hand_index)
3. Game enters `SelectMaterial` phase — valid first materials highlight on field
4. Player clicks first material
5. Valid second materials highlight
6. Player clicks second material → backend resolves DNA digivolution

**Hatching / Moving from Breeding:**
- Action bar shows "Hatch" button when in Breeding phase with eggs available (action 60)
- Action bar shows "Move" button when breeding area has a L3+ digimon (action 61)

**Passing / Declining:**
- "Pass" button always visible during player's turn, sends action 62
- During selection phases with `is_optional`, "Decline" button sends action 62

### 1.4 Selection Phase UI

The engine now has 10 selection phases beyond Main/Breeding. Each needs specific UI treatment:

| Phase | UI Behavior |
|-------|-------------|
| `SelectTarget` (5) | Highlight valid permanents on field (own + opponent). Player clicks one. |
| `SelectMaterial` (6) | Two-step: highlight valid first materials → click → highlight valid second materials → click. Used by DNA Digivolve. |
| `BlockTiming` (7) | Show opponent's valid blockers highlighted. "Block with [name]" buttons + "Decline" (62). |
| `CounterTiming` (8) | Show opponent's blast digivolve options. "Blast Digivolve [card]" buttons + "Decline" (62). |
| `SelectTrash` (9) | Open `TrashBrowser` modal. Player clicks a card from trash. Valid indices 130-179. |
| `SelectSource` (10) | Open `StackBrowser` modal showing digivolution stack. Player clicks a source card. |
| `SelectHand` (11) | Highlight selectable hand cards. Player clicks one. Valid indices 0-29. |
| `SelectReveal` (12) | Populate `RevealedCardsZone` with face-up cards. Player clicks one. Valid indices 30-39. |
| `SelectEffectChoice` (13) | Show `EffectChoicePanel` with 2+ buttons describing each branch. Valid indices 1000-1009. |
| `SelectSecurity` (14) | Open `SecurityBrowser` modal. Player clicks a security card. Valid indices 40-59. |

A `SelectionOverlay` component wraps the board during any selection phase, dimming non-interactive elements and showing a prompt ("Choose a target", "Select a card from trash", etc.).

### 1.5 Visual Effects (Phase 2)

These are nice-to-have and can be added incrementally:
- Attack arrows (SVG lines between attacker and target, with pulsing animation)
- Card play animation (hand → field slide)
- Suspend/unsuspend tilt animation (CSS transform rotate 30deg)
- DP modifier badges (green +, red -)
- Security check flip animation
- Turn transition overlay
- Keyword grant/remove flash effects

### 1.6 State Management (Zustand)

```typescript
// stores/gameStore.ts
interface GameStore {
  // Connection
  gameId: string | null;
  wsConnected: boolean;

  // Game state (from server to_ui_json)
  turnCount: number;
  currentPhase: GamePhase;  // 0-14
  currentPlayer: 1 | 2;
  memoryGauge: number;
  isGameOver: boolean;
  winner: number | null;
  player1: PlayerState;
  player2: PlayerState;
  revealedCards: CardInfo[];

  // Selection state (from server)
  pendingSelection: {
    phase: GamePhase;
    validIndices: number[];
    isOptional: boolean;
    prompt: string;
    selectingPlayer: 1 | 2;
  } | null;
  pendingAttack: {
    attackerSlot: number;
    targetSlot: number;
  } | null;

  // Action mask (from server, 2120 elements)
  actionMask: number[];

  // Local UI state
  selectedHandCard: number | null;
  selectedAttacker: number | null;
  selectedMaterials: number[];  // for DNA two-step
  hoveredCard: CardInfo | null;
  logs: string[];

  // Actions
  setGameState: (state: ServerGameState) => void;
  setActionMask: (mask: number[]) => void;
  selectHandCard: (index: number | null) => void;
  selectAttacker: (index: number | null) => void;
  setHoveredCard: (card: CardInfo | null) => void;
  appendLogs: (logs: string[]) => void;
}

interface PlayerState {
  handCount: number;
  handIds: string[];       // only for "our" player in PvP
  securityCount: number;
  securityIds: string[];   // only for own player
  deckCount: number;
  eggDeckCount: number;
  battleArea: PermanentInfo[];
  breedingArea: PermanentInfo | null;
  trashIds: string[];
}

interface PermanentInfo {
  topCardId: string;
  topCardName: string;
  dp: number;
  level: number;
  isSuspended: boolean;
  sourceCount: number;
  sources: SourceInfo[];          // full digivolution stack
  linkedCardIds: string[];        // [TS] option cards
  keywords: string[];             // ["rush", "blocker", "jamming", ...]
  securityAttackModifier: number; // +1, -1, etc.
  turnPlayed: number;
}

interface SourceInfo {
  cardId: string;
  optState: number;     // -1 = no OPT, 0 = exhausted, 1 = available
  dpContribution: number;
}
```

### 1.7 Game Phases Reference

All 15 phases the UI must handle (from `GamePhase` enum):

| Value | Phase | UI Mode |
|-------|-------|---------|
| 0 | Start | Automatic (no UI) |
| 1 | Draw | Automatic (no UI) |
| 2 | Breeding | Action bar: Hatch / Move / Pass |
| 3 | Main | Full interaction: play, attack, digivolve, DNA, effects |
| 4 | End | Automatic (no UI) |
| 5 | SelectTarget | Selection overlay on field |
| 6 | SelectMaterial | Two-step field selection |
| 7 | BlockTiming | Opponent: blocker selection or decline |
| 8 | CounterTiming | Opponent: blast digivolve or decline |
| 9 | SelectTrash | Trash browser modal |
| 10 | SelectSource | Stack browser modal |
| 11 | SelectHand | Hand card highlighting |
| 12 | SelectReveal | Revealed cards zone |
| 13 | SelectEffectChoice | Effect choice buttons |
| 14 | SelectSecurity | Security browser modal |

---

## 2. Lobby & Multiplayer

### 2.1 Game Modes

| Mode | Description | How it starts |
|------|-------------|---------------|
| **vs Agent** | Human vs AI agent | Solo — pick agent + deck, start immediately |
| **Private Room** | Human vs Human, invite via code | Host creates room → gets 4-char code → guest joins with code |
| **Quick Match** | Human vs Human, random opponent | Join queue → server pairs two players |
| **Spectate** | Watch a live game | Enter room code or pick from active games list |

### 2.2 Lobby Page

```
┌──────────────────────────────────────────────────────────────┐
│  DIGIMON TCG SIMULATOR                                       │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  VS AGENT   │  │ PLAY ONLINE │  │  REPLAYS    │         │
│  │             │  │             │  │             │         │
│  │ Practice    │  │ Challenge a │  │ Watch past  │         │
│  │ against AI  │  │ friend or   │  │ games       │         │
│  │             │  │ find match  │  │             │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│                                                              │
│  ── Play Online ──────────────────────────────────────────  │
│                                                              │
│  Deck: [Red Starter ST1 ▼]                                  │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────┐                 │
│  │  CREATE ROOM     │  │  JOIN ROOM       │                 │
│  │                  │  │                  │                 │
│  │ Get a room code  │  │  Code: [____]    │                 │
│  │ to share with a  │  │                  │                 │
│  │ friend           │  │  [Join]          │                 │
│  │                  │  │                  │                 │
│  │ [Create]         │  │                  │                 │
│  └──────────────────┘  └──────────────────┘                 │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────────────────────┐ │
│  │  QUICK MATCH     │  │  ACTIVE ROOMS (spectate)         │ │
│  │                  │  │                                  │ │
│  │  [Find Match]    │  │  ABCD - Player1 vs Player2  [W]  │ │
│  │  Searching...    │  │  EFGH - Player3 vs (waiting) [W] │ │
│  │                  │  │  IJKL - Player4 vs Player5  [W]  │ │
│  └──────────────────┘  └──────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### 2.3 Room Lifecycle

```
Host creates room
  |
  +-> Room enters "waiting" state
  |   Room code generated (e.g., "ABCD")
  |   Host selects deck, sees "Waiting for opponent..."
  |
  +-> Guest joins with code (or Quick Match pairs them)
  |   Guest selects deck
  |   Both players see opponent's name/avatar
  |
  +-> Both players ready -> Game starts
  |   Room state transitions to "playing"
  |   Both clients connect to game WebSocket
  |
  +-> Game ends -> Results shown
  |   Option to rematch (swap first player) or return to lobby
  |
  +-> Room cleaned up after both players leave or timeout
```

### 2.4 Room Codes

- **4 uppercase alphanumeric characters** (e.g., `ABCD`, `X7KM`) — 36^4 = ~1.7M combinations, plenty for concurrent rooms
- Generated server-side, collision-checked against active rooms
- Codes are reusable after room is closed
- Codes are case-insensitive on input (normalized to uppercase)

### 2.5 Waiting Room UI

```
+--------------------------------------------------------------+
|  Room: ABCD                              [Copy Code] [Leave] |
|                                                              |
|  +---------------------+    +---------------------+         |
|  |  PLAYER 1 (you)     |    |  PLAYER 2           |         |
|  |                     |    |                     |         |
|  |  Deck: Red Starter  |    |  Waiting...         |         |
|  |  Ready              |    |                     |         |
|  |                     |    |                     |         |
|  +---------------------+    +---------------------+         |
|                                                              |
|  Share this code with your opponent: ABCD                    |
|                                                              |
|  [Start Game]  (disabled until both players ready)           |
+--------------------------------------------------------------+
```

### 2.6 PvP Game Differences

In Human vs Human, both players are connected via WebSocket. Key differences from Human vs Agent:

| Aspect | vs Agent | vs Human (PvP) |
|--------|----------|-----------------|
| Connections | 1 WebSocket (human player) | 2 WebSockets (one per player) |
| Hidden info | Agent sees full state; human sees own hand only | Each player sees only their own hand |
| Turn flow | Agent moves are instant after human passes | Both players wait for opponent |
| Action source | Human clicks UI or agent auto-plays | Both players click UI |
| Disconnection | Agent never disconnects | Handle reconnection, timeout, forfeit |

**State filtering**: The server must send **player-specific views** — each client only receives their own hand cards and security contents. Opponent hand is shown as face-down card backs with a count. This requires a new `to_player_json(player_id)` method on `Game`.

### 2.7 Spectator Mode

Spectators connect to a game's WebSocket with a `role=spectator` parameter. They receive a neutral view with both hands hidden. Spectators cannot send actions.

### 2.8 Lobby WebSocket: `/ws/lobby`

A persistent WebSocket for lobby state. All connected clients receive room list updates.

**Client -> Server:**
```json
{"type": "create_room", "deck_ids": ["ST1-01", ...], "player_name": "Alice"}

{"type": "join_room", "room_code": "ABCD", "deck_ids": ["BT14-001", ...], "player_name": "Bob"}

{"type": "leave_room"}

{"type": "set_ready", "ready": true}

{"type": "queue_quickmatch", "deck_ids": ["ST1-01", ...], "player_name": "Alice"}

{"type": "cancel_quickmatch"}
```

**Server -> Client:**
```json
{
  "type": "room_created",
  "room_code": "ABCD",
  "room": { "code": "ABCD", "host": "Alice", "guest": null, "status": "waiting" }
}

{
  "type": "player_joined",
  "room": { "code": "ABCD", "host": "Alice", "guest": "Bob", "status": "waiting" }
}

{
  "type": "game_starting",
  "room_code": "ABCD",
  "game_id": "uuid",
  "your_player_id": 1
}

{
  "type": "room_list",
  "rooms": [
    {"code": "ABCD", "host": "Alice", "guest": "Bob", "status": "playing"},
    {"code": "EFGH", "host": "Charlie", "guest": null, "status": "waiting"}
  ]
}

{
  "type": "quickmatch_found",
  "game_id": "uuid",
  "opponent_name": "Bob",
  "your_player_id": 1
}

{
  "type": "player_disconnected",
  "player_name": "Bob"
}

{
  "type": "player_reconnected",
  "player_name": "Bob"
}
```

### 2.9 PvP Game WebSocket Changes

The existing `/ws/game/{game_id}` WebSocket is extended for PvP:

**Client -> Server** (same as before, but with player auth):
```json
{"type": "connect", "player_id": 1, "reconnect_token": "..."}
{"type": "action", "action_id": 60}
```

**Server -> Client** (player-specific state):
```json
{
  "type": "state_update",
  "state": { /* filtered to_player_json() */ },
  "action_mask": [0, 0, 1, ...],
  "logs": ["Player 1 plays Agumon"],
  "is_your_turn": true,
  "is_game_over": false
}

{
  "type": "opponent_action",
  "action_description": "Opponent plays a card from hand",
  "logs": ["Player 2 plays a Digimon"]
}

{
  "type": "waiting_for_opponent"
}
```

### 2.10 Disconnection & Reconnection

- Each player gets a **reconnect token** when the game starts (random UUID stored server-side)
- If a WebSocket drops, the server keeps the game alive for **5 minutes**
- Player can reconnect using the token and resume from current state
- If timeout expires, disconnected player **forfeits**
- Opponent sees "Opponent disconnected... waiting for reconnection" message
- Spectators see a disconnection indicator

### 2.11 Rematch Flow

After a game ends:

```json
// Player sends
{"type": "request_rematch"}

// Opponent receives
{"type": "rematch_requested", "from": "Alice"}

// Opponent accepts
{"type": "accept_rematch"}

// Both receive
{"type": "game_starting", "game_id": "new-uuid", "your_player_id": 2}
// (player IDs swap so first-player alternates)
```

---

## 3. Replay Viewer — Agent vs Agent Playback

### 3.1 Concept

Record full game state snapshots at every action during Agent vs Agent games. The replay viewer loads the recording and lets the user scrub through it like a video timeline.

### 3.2 Recording Format

Each recorded game is a JSON file. Frames use the enhanced `to_ui_json()` so keyword badges, linked cards, and selection phases are visible during playback.

```json
{
  "metadata": {
    "replay_id": "uuid",
    "timestamp": "ISO-8601",
    "deck1_ids": ["ST1-01", ...],
    "deck2_ids": ["BT14-001", ...],
    "agent1": "greedy",
    "agent2": "maskable_ppo_v3",
    "winner": 1,
    "total_turns": 42,
    "total_actions": 187
  },
  "frames": [
    {
      "frame_id": 0,
      "action_id": null,
      "action_description": "Game Start",
      "state": { /* full to_ui_json() snapshot */ },
      "logs": ["Game started. Player 1 goes first."]
    },
    {
      "frame_id": 1,
      "action_id": 60,
      "action_description": "Player 1: Hatch",
      "player": 1,
      "state": { /* to_ui_json() */ },
      "logs": ["Player 1 hatches ST1-01 Koromon"]
    }
  ]
}
```

### 3.3 Replay UI

The replay viewer reuses the same `GameBoard` component from the interactive game, but in read-only mode with playback controls:

```
+--------------------------------------------------------+
|  Same board layout as interactive game (read-only)     |
|  Both players' hands visible (no hidden info)          |
|                                                        |
|  +--------------------------------------------------+  |
|  | <<  <  >  >>  |  Frame 47/187  |  1x  2x  4x    |  |
|  | Timeline scrubber =========O=====================|  |
|  +--------------------------------------------------+  |
|                                                        |
|  +--------------------------------------------------+  |
|  |  Action Log (synced to current frame)            |  |
|  |  > Player 1: Play Agumon (cost 3, memory 4->1)  |  |
|  |  > Player 1: Attack with Greymon -> Security     |  |
|  |  > Player 1: Pass turn                           |  |
|  |  > Player 2: Hatch Tokomon                       |  |
|  +--------------------------------------------------+  |
|                                                        |
|  Metadata: Agent1=greedy vs Agent2=ppo_v3 | Winner: P1|
+--------------------------------------------------------+
```

**Controls:**
- Play/Pause with configurable speed (1x, 2x, 4x, 0.5x)
- Step forward/backward one frame
- Jump to start/end
- Scrubber bar to seek to any frame
- Auto-scroll log to current frame
- Both players' hands are visible (since it's a replay, no hidden information)

### 3.4 Replay State Store

```typescript
// stores/replayStore.ts
interface ReplayStore {
  replay: ReplayData | null;
  currentFrame: number;
  isPlaying: boolean;
  playbackSpeed: number; // 0.5, 1, 2, 4

  loadReplay: (data: ReplayData) => void;
  setFrame: (frame: number) => void;
  stepForward: () => void;
  stepBackward: () => void;
  togglePlay: () => void;
  setSpeed: (speed: number) => void;
}
```

---

## 4. Admin Dashboard

### 4.1 Pages

| Page | Purpose |
|------|---------|
| **Agent List** | View all trained agents, their type, training status, win rates |
| **Agent Detail** | View agent config, training history, play sample games |
| **Training Jobs** | Launch new training runs, monitor progress, stop/resume |
| **Matchup Matrix** | Run Agent A vs Agent B simulations, see win rates in a grid |
| **Replay Browser** | List and filter recorded replays by agents, date, winner |
| **Card Database** | Browse the 3,651-card database with set/color/kind filters |

### 4.2 Agent Management

The backend already has `pilot_training.py` with `OpponentWrapper`, `WinRateCallback`, and CLI args (`--timesteps`, `--opponent`, `--self-play`). The admin UI wraps this.

Available agent types: `greedy` (built-in), `random` (built-in), `maskable_ppo` (via pilot_training.py).

```
+-----------------------------------------------------+
|  Agents                                  [+ New Agent]|
|----------------------------------------------------- |
|  Name          | Type        | Status  | Win Rate    |
|  greedy_v1     | Greedy      | Ready   | 43.2%       |
|  random_v1     | Random      | Ready   | 12.4%       |
|  ppo_v3        | MaskablePPO | Ready   | 61.8%       |
|  ppo_v4        | MaskablePPO | Training| --          |
+-----------------------------------------------------+
```

### 4.3 Training Job Panel

Training launch wraps `pilot_training.py` as a subprocess rather than reimplementing. Metrics come from `WinRateCallback` output.

```
+-----------------------------------------------------+
|  New Training Job                                    |
|                                                      |
|  Agent Type:   [MaskablePPO v]                       |
|  Base Agent:   [ppo_v3 v] (or "from scratch")       |
|  Opponent:     [greedy v] / [random v] / [self-play] |
|  Timesteps:    [1,000,000    ]                       |
|  Learning Rate:[0.0003       ]                       |
|                                                      |
|  [Launch Training]                                   |
|                                                      |
|  -- Active Jobs --                                   |
|  ppo_v4 | 340k/1M steps | ========---- 34% | [Stop] |
|  Reward curve: [sparkline chart]                     |
|  Win rate: 54.2% (eval every 10k steps)              |
+-----------------------------------------------------+
```

### 4.4 Matchup Matrix

```
+----------------------------------------------+
|  Matchup Matrix (100 games each)             |
|                                              |
|              |greedy|random|ppo_v3|ppo_v4    |
|  greedy      |  --  | 78%  | 38%  | 41%     |
|  random      | 22%  |  --  | 12%  | 15%     |
|  ppo_v3      | 62%  | 88%  |  --  | 52%     |
|  ppo_v4      | 59%  | 85%  | 48%  |  --     |
|                                              |
|  [Run Full Matrix]  [Export CSV]             |
+----------------------------------------------+
```

---

## 5. API Endpoints

### 5.1 Existing Endpoints (keep as-is)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/` | Health check |
| POST | `/simulate` | Batch simulation |
| POST | `/game/create` | Create game session |
| POST | `/game/{id}/action` | Execute action |
| POST | `/game/{id}/step` | Advance interactive game |
| GET | `/game/{id}/state` | Get game state |
| GET | `/game/{id}/mask` | Get action mask |
| GET | `/game/{id}/log` | Get game log |
| DELETE | `/game/{id}` | Delete session |

### 5.2 New Endpoints — Game UI

#### WebSocket: `/ws/game/{game_id}`

Replace polling with a WebSocket connection for real-time game state updates. Supports PvA (1 connection) and PvP (2 connections + spectators).

**Client -> Server messages:**
```json
{"type": "connect", "player_id": 1, "reconnect_token": "..."}
{"type": "action", "action_id": 60}
{"type": "step"}
```

**Server -> Client messages:**
```json
{
  "type": "state_update",
  "state": { /* to_ui_json() or to_player_json() */ },
  "action_mask": [0, 0, 1, ...],
  "logs": ["Player 1 hatches..."],
  "pending_selection": {
    "phase": "SelectTarget",
    "valid_indices": [100, 101, 112, 113],
    "is_optional": true,
    "prompt": "Choose a Digimon to return to hand"
  },
  "is_your_turn": true,
  "is_game_over": false
}
```

```json
{
  "type": "game_over",
  "winner": 1,
  "state": { /* final state */ },
  "logs": ["Player 1 wins!"]
}
```

#### `GET /cards`

Return the card database with pagination and filtering (3,651 cards).

```
GET /cards?set=BT14&color=Red&kind=Digimon&q=Agumon&page=1&limit=50
```

```json
{
  "cards": [
    {
      "card_id": "BT14-001",
      "card_name": "Koromon",
      "card_color": "Red",
      "card_kind": "DigiEgg",
      "level": 2,
      "dp": null,
      "cost": 0,
      "card_text": "...",
      "image_url": null
    }
  ],
  "total": 3651,
  "page": 1,
  "page_size": 50
}
```

#### `GET /cards/{card_id}/image`

Return card image. Initially can return a placeholder; later integrate with card image CDN or local assets.

### 5.3 New Endpoints — Lobby & Multiplayer

#### `POST /rooms`

Create a private room.

```json
// Request
{
  "player_name": "Alice",
  "deck_ids": ["ST1-01", "ST1-02", "..."]
}

// Response
{
  "room_code": "ABCD",
  "room": {
    "code": "ABCD",
    "host": "Alice",
    "guest": null,
    "status": "waiting",
    "created_at": "2026-02-06T..."
  }
}
```

#### `POST /rooms/{code}/join`

Join an existing room by code.

```json
// Request
{
  "player_name": "Bob",
  "deck_ids": ["BT14-001", "BT14-002", "..."]
}

// Response
{
  "room_code": "ABCD",
  "room": {
    "code": "ABCD",
    "host": "Alice",
    "guest": "Bob",
    "status": "waiting"
  }
}
```

#### `GET /rooms`

List public/active rooms (for spectator browsing).

```json
{
  "rooms": [
    {"code": "ABCD", "host": "Alice", "guest": "Bob", "status": "playing"},
    {"code": "EFGH", "host": "Charlie", "guest": null, "status": "waiting"}
  ]
}
```

#### `POST /rooms/{code}/start`

Host starts the game (both players must be ready).

```json
// Response
{
  "game_id": "uuid",
  "room_code": "ABCD"
}
```

#### `DELETE /rooms/{code}`

Leave / close a room.

#### `POST /quickmatch/queue`

Join the quickmatch queue.

```json
// Request
{
  "player_name": "Alice",
  "deck_ids": ["ST1-01", "..."]
}

// Response
{
  "queue_position": 1,
  "status": "searching"
}
```

#### `DELETE /quickmatch/queue`

Leave the quickmatch queue.

#### WebSocket: `/ws/lobby`

Persistent lobby connection for real-time room updates and quickmatch notifications. See section 2.8 for the full message protocol.

### 5.4 New Endpoints — Replay System

#### `POST /replay/record`

Start recording a game. This creates a headless game that records every frame.

```json
// Request
{
  "deck1": ["ST1-01", "..."],
  "deck2": ["BT14-001", "..."],
  "agent1": "greedy",
  "agent2": "ppo_v3",
  "max_turns": 200
}

// Response
{
  "replay_id": "uuid",
  "status": "recording"
}
```

#### `POST /replay/record/{replay_id}/run`

Execute the recorded game to completion (agents play automatically).

```json
// Response
{
  "replay_id": "uuid",
  "status": "complete",
  "winner": 1,
  "total_frames": 187
}
```

#### `GET /replays`

List available replays with metadata filtering.

```
GET /replays?agent1=ppo_v3&agent2=greedy&limit=20&offset=0
```

```json
{
  "replays": [
    {
      "replay_id": "uuid",
      "timestamp": "2026-02-06T...",
      "agent1": "greedy",
      "agent2": "ppo_v3",
      "winner": 1,
      "total_turns": 42,
      "total_frames": 187
    }
  ],
  "total": 150
}
```

#### `GET /replays/{replay_id}`

Get full replay data (metadata + all frames).

#### `DELETE /replays/{replay_id}`

Delete a replay.

### 5.5 New Endpoints — Admin / Agent Management

#### `GET /agents`

List all registered agents.

```json
{
  "agents": [
    {
      "agent_id": "greedy_v1",
      "agent_type": "greedy",
      "status": "ready",
      "model_path": null,
      "created_at": "2026-01-15T...",
      "config": {}
    },
    {
      "agent_id": "ppo_v3",
      "agent_type": "maskable_ppo",
      "status": "ready",
      "model_path": "models/ppo_v3.zip",
      "created_at": "2026-02-01T...",
      "config": {
        "learning_rate": 0.0003,
        "total_timesteps": 1000000,
        "trained_timesteps": 1000000
      }
    }
  ]
}
```

#### `POST /agents`

Register a new agent.

```json
// Request
{
  "agent_id": "ppo_v5",
  "agent_type": "maskable_ppo",
  "config": {
    "learning_rate": 0.0003,
    "n_steps": 2048,
    "batch_size": 64
  },
  "base_agent_id": "ppo_v3"
}
```

#### `GET /agents/{agent_id}`

Get agent details including training history.

#### `DELETE /agents/{agent_id}`

Remove an agent.

#### `POST /training/start`

Launch a training job (wraps `pilot_training.py` as a subprocess).

```json
// Request
{
  "agent_id": "ppo_v5",
  "total_timesteps": 1000000,
  "opponent": "greedy",
  "self_play": false,
  "checkpoint_interval": 100000
}

// Response
{
  "job_id": "uuid",
  "status": "running"
}
```

#### `GET /training/jobs`

List active and completed training jobs.

```json
{
  "jobs": [
    {
      "job_id": "uuid",
      "agent_id": "ppo_v5",
      "status": "running",
      "progress": {
        "current_timesteps": 340000,
        "total_timesteps": 1000000,
        "mean_reward": 0.23,
        "mean_episode_length": 87,
        "win_rate": 0.542
      },
      "started_at": "2026-02-06T..."
    }
  ]
}
```

#### `POST /training/jobs/{job_id}/stop`

Stop a running training job and save the current checkpoint.

#### `GET /training/jobs/{job_id}/metrics`

Get training metrics (from `WinRateCallback`) for charting.

```json
{
  "metrics": [
    {"timestep": 10000, "mean_reward": -0.5, "mean_ep_length": 120, "win_rate": 0.32},
    {"timestep": 20000, "mean_reward": -0.3, "mean_ep_length": 105, "win_rate": 0.41}
  ]
}
```

#### `POST /matchup`

Run a matchup between two agents.

```json
// Request
{
  "agent1_id": "ppo_v3",
  "agent2_id": "greedy_v1",
  "num_games": 100
}

// Response
{
  "matchup_id": "uuid",
  "status": "running"
}
```

#### `GET /matchup/{matchup_id}`

Get matchup results.

```json
{
  "agent1_id": "ppo_v3",
  "agent2_id": "greedy_v1",
  "status": "complete",
  "results": {
    "agent1_wins": 62,
    "agent2_wins": 38,
    "draws": 0,
    "total_games": 100,
    "agent1_win_rate": 0.62
  }
}
```

---

## 6. Frontend Architecture

### 6.1 Directory Structure

```
frontend/
├── index.html
├── vite.config.ts
├── tsconfig.json
├── package.json
├── public/
│   └── card-back.png
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── api/
│   │   ├── client.ts            # Axios instance, base URL config
│   │   ├── gameApi.ts           # REST endpoints for game
│   │   ├── cardApi.ts           # Card database with pagination
│   │   ├── replayApi.ts         # Replay CRUD
│   │   ├── agentApi.ts          # Agent management
│   │   └── trainingApi.ts       # Training jobs
│   ├── ws/
│   │   ├── gameSocket.ts        # WebSocket connection for game
│   │   └── lobbySocket.ts       # WebSocket connection for lobby
│   ├── stores/
│   │   ├── gameStore.ts         # Interactive game state (Zustand)
│   │   ├── lobbyStore.ts        # Room list, queue status, current room
│   │   ├── replayStore.ts       # Replay playback state
│   │   ├── uiStore.ts           # UI state (selected cards, modals)
│   │   └── adminStore.ts        # Agent/training state
│   ├── pages/
│   │   ├── HomePage.tsx         # Landing: new game / play online / replays / admin
│   │   ├── LobbyPage.tsx        # Room creation, join, quickmatch, active rooms
│   │   ├── WaitingRoomPage.tsx  # Pre-game room (deck select, ready up)
│   │   ├── GamePage.tsx         # Interactive game board (PvA and PvP)
│   │   ├── ReplayPage.tsx       # Replay viewer
│   │   ├── ReplayListPage.tsx   # Browse/filter replays
│   │   ├── AdminPage.tsx        # Agent list + training dashboard
│   │   └── MatchupPage.tsx      # Matchup matrix
│   ├── components/
│   │   ├── board/
│   │   │   ├── GameBoard.tsx    # Root board layout (shared by game + replay)
│   │   │   ├── PlayerHalf.tsx   # One player's zones
│   │   │   ├── BattleArea.tsx   # Field permanents (12 slots)
│   │   │   ├── PermanentSlot.tsx# Single permanent with stack
│   │   │   ├── HandZone.tsx     # Player hand
│   │   │   ├── SecurityStack.tsx
│   │   │   ├── DeckPile.tsx
│   │   │   ├── EggDeck.tsx
│   │   │   ├── BreedingArea.tsx
│   │   │   ├── TrashPile.tsx
│   │   │   ├── MemoryGauge.tsx
│   │   │   ├── RevealedCardsZone.tsx  # Face-up revealed cards row
│   │   │   ├── LinkedCards.tsx         # Sideways option cards on permanent
│   │   │   └── KeywordBadges.tsx      # Keyword icons on permanents
│   │   ├── lobby/
│   │   │   ├── CreateRoom.tsx   # Create private room form
│   │   │   ├── JoinRoom.tsx     # Join by code input
│   │   │   ├── QuickMatchButton.tsx # Queue for random match
│   │   │   ├── ActiveRoomsList.tsx  # Spectatable games list
│   │   │   └── WaitingRoom.tsx  # Pre-game player cards + ready state
│   │   ├── game/
│   │   │   ├── ActionBar.tsx    # Contextual action buttons
│   │   │   ├── PhaseIndicator.tsx
│   │   │   ├── GameLog.tsx
│   │   │   ├── CardDetail.tsx   # Sidebar card preview
│   │   │   ├── SelectionOverlay.tsx   # Phase-aware selection dimming + prompt
│   │   │   ├── DnaDigivolveFlow.tsx   # Multi-step DNA material picker
│   │   │   ├── TrashBrowser.tsx       # Modal for SelectTrash phase
│   │   │   ├── StackBrowser.tsx       # Modal for SelectSource phase
│   │   │   ├── EffectChoicePanel.tsx  # Buttons for SelectEffectChoice
│   │   │   └── SecurityBrowser.tsx    # Modal for SelectSecurity phase
│   │   ├── replay/
│   │   │   ├── PlaybackControls.tsx
│   │   │   ├── Timeline.tsx
│   │   │   └── FrameLog.tsx
│   │   ├── admin/
│   │   │   ├── AgentList.tsx
│   │   │   ├── AgentCard.tsx
│   │   │   ├── TrainingJobForm.tsx
│   │   │   ├── TrainingJobList.tsx
│   │   │   ├── MetricsChart.tsx
│   │   │   └── MatchupMatrix.tsx
│   │   └── shared/
│   │       ├── Card.tsx         # Card renderer (image + overlays)
│   │       ├── CardStack.tsx    # Digivolution stack view
│   │       └── Modal.tsx
│   ├── hooks/
│   │   ├── useGameSocket.ts    # WebSocket hook for game
│   │   ├── useLobbySocket.ts   # WebSocket hook for lobby
│   │   ├── useActionMask.ts    # Parse mask into actionable UI state
│   │   └── useReplayPlayer.ts  # Playback timer logic
│   ├── utils/
│   │   ├── actionDecoder.ts    # Decode action ID -> human description
│   │   └── constants.ts        # Action ranges, phase names, keyword list
│   └── types/
│       ├── game.ts             # GameState, PlayerState, PermanentInfo
│       ├── replay.ts           # ReplayData, Frame
│       └── admin.ts            # Agent, TrainingJob, Matchup
```

### 6.2 Key Libraries

| Library | Purpose |
|---------|---------|
| React 19 | UI framework |
| TypeScript | Type safety |
| Vite | Build tool |
| Zustand | State management (simpler than Redux for this scope) |
| React Router | Page navigation |
| Axios | REST API calls |
| native WebSocket | Real-time game updates (no library needed) |
| Recharts or Chart.js | Training metrics charts |
| Tailwind CSS | Styling (utility-first, fast iteration) |

### 6.3 Card Rendering

Cards need images. Options in priority order:

1. **Placeholder cards** — Colored rectangles with card name, level, DP, cost text. Color-coded by CardColor. Good enough for development.
2. **Local assets** — Download card images and serve from `/public/cards/`.
3. **CDN proxy** — Proxy card images from an external source through the backend.

The `Card` component should accept a `cardId` and render whatever is available, falling back gracefully.

---

## 7. Implementation Phases

### Phase 1: Foundation
- Set up React + Vite + TypeScript project in `frontend/`
- Add CORS middleware to FastAPI
- Implement `GET /cards` endpoint with pagination (3,651 cards)
- Build `Card` component with placeholder rendering (color-coded by CardColor)
- Build `GameBoard` layout with all zones (static/mock data)
- Build `MemoryGauge`, `PhaseIndicator`, `GameLog` components
- Reference `TENSOR_SPEC.md` and `ACTION_SPEC.md` for all mappings

### Phase 2: Interactive Game (Human vs Agent)
- Implement WebSocket endpoint `/ws/game/{game_id}`
- Build `useGameSocket` hook
- Implement click-to-act interaction flow for Main + Breeding phases
- Build `ActionBar` with contextual buttons derived from action mask
- Build `useActionMask` hook (translates 2120 mask -> UI-friendly action list)
- Build all 10 selection phase UIs (SelectionOverlay, TrashBrowser, StackBrowser, EffectChoicePanel, SecurityBrowser, DnaDigivolveFlow)
- Implement keyword badge rendering on permanents
- Implement linked card display
- Implement revealed cards zone
- Connect `GamePage` end-to-end: create game -> play -> game over

### Phase 3: Multiplayer (PvP)
- Implement `/rooms` CRUD endpoints and room code generation
- Implement `/ws/lobby` WebSocket for real-time room updates
- Add `to_player_json(player_id)` to `Game` for player-specific state filtering
- Build `LobbyPage`, `WaitingRoomPage` components
- Extend `/ws/game/{game_id}` for two-player connections
- Implement quickmatch queue with pairing logic
- Add disconnection handling, reconnect tokens, forfeit timeout
- Add spectator mode (read-only WebSocket connection)
- Add rematch flow

### Phase 4: Replay System
- Implement replay recording in backend (wrap HeadlessGame, use `to_ui_json()`)
- Implement `/replays` CRUD endpoints
- Build `PlaybackControls` + `Timeline` components
- Build `ReplayPage` reusing `GameBoard` in read-only mode
- Build `ReplayListPage` with filtering

### Phase 5: Admin Dashboard
- Implement `/agents` CRUD endpoints
- Implement `/training/start` wrapping `pilot_training.py` as subprocess
- Expose `WinRateCallback` metrics via `/training/jobs/{id}/metrics`
- Build agent list and detail views
- Build training job launch form
- Build metrics chart (reward curve + win rate over timesteps)
- Implement `/matchup` endpoints
- Build matchup matrix view

### Phase 6: Polish
- Card images (replace placeholders)
- Attack arrows (SVG overlay with pulsing animation)
- Suspend/unsuspend animations
- Sound effects
- Mobile-responsive layout
- Error handling and reconnection logic

---

## 8. Backend Changes Summary

### New `to_ui_json()` method on `Game`

The current `to_json()` lacks data the UI needs. A new `to_ui_json()` (or enhanced `to_json()`) must include:

**Per-permanent (extends existing `BattleArea` entries):**
- `Keywords`: list of active keyword strings (rush, blocker, jamming, piercing, retaliation, blitz, reboot, collision, evade, armor_purge, barrier)
- `SecurityAttackModifier`: int (+1, -1, etc.)
- `LinkedCardIds`: list of attached option card IDs
- `Sources`: full digivolution stack (card_id, opt_state, dp_contribution per source)
- `TurnPlayed`: int (for rush/summoning sickness display)

**Game-level (new fields):**
- `RevealedCards`: list of {card_id, card_name} for revealed cards zone
- `PendingSelection`: {phase, valid_indices, is_optional, prompt, selecting_player} or null
- `PendingAttack`: {attacker_slot, target_slot} or null (for attack arrow rendering)

**Player-level (extends existing):**
- `TrashIds`: list of card IDs in trash
- `SecurityIds`: list of card IDs (own player only, hidden for opponent)
- `EggDeckCount`: int

### New `to_player_json(player_id)` method

Same as `to_ui_json()` but filters hidden information:
- Opponent's hand shows only count, not card IDs
- Opponent's security shows only count, not card IDs
- Own hand and security are fully visible

### New files to create:

| File | Purpose |
|------|---------|
| `digimon_gym/api_ws.py` | WebSocket game handler (PvA + PvP + spectator) |
| `digimon_gym/lobby.py` | Room management, quickmatch queue, lobby WebSocket |
| `digimon_gym/replay.py` | Replay recording and storage |
| `digimon_gym/agents/registry.py` | Agent registration (wraps pilot_training.py) |
| `digimon_gym/training/manager.py` | Training job lifecycle (subprocess wrapper) |
| `digimon_gym/training/metrics.py` | Metrics collection from WinRateCallback |

### Modifications to existing files:

| File | Changes |
|------|---------|
| `digimon_gym/api.py` | Add card (paginated), room, replay, agent, training, matchup endpoints; mount WebSockets; add CORS middleware |
| `digimon_gym/engine/game.py` | Add `to_ui_json()`, `to_player_json(player_id)`, `action_description(action_id)` |
| `digimon_gym/engine/runners/headless_game.py` | Add replay recording hooks |
| `digimon_gym/engine/data/card_database.py` | Add `to_dict()`, `search(set, color, kind, query)` for card API |

### Data storage:

For the pre-alpha stage, use **file-based storage** (JSON files):
- `data/replays/` — One JSON file per replay
- `data/agents.json` — Agent registry
- `data/training_jobs.json` — Training job history
- `models/` — Saved model files (`.zip` for SB3)

Migrate to SQLite or PostgreSQL when needed.

---

## 9. Action Mask -> UI Mapping

The 2120-element action mask needs to be translated into UI-friendly actions. The `useActionMask` hook does this, referencing `ACTION_SPEC.md`:

```typescript
interface ParsedActions {
  // Main phase actions
  canPlay: { handIndex: number; cardId: string }[];          // 0-29
  canTrash: { handIndex: number; cardId: string }[];          // 30-59
  canHatch: boolean;                                          // 60
  canMoveFromBreeding: boolean;                               // 61
  canPass: boolean;                                           // 62
  canDnaDigivolve: { handIndex: number; cardId: string }[];   // 63-92
  canAttack: {                                                // 100-399
    attackerSlot: number;
    targets: { slot: number; isSecurity: boolean }[];
  }[];
  canDigivolve: { handIndex: number; targets: number[] }[];   // 400-999
  canActivateEffect: { permanentSlot: number; effectIdx: number }[]; // 1000-1999
  canSelectSource: { fieldSlot: number; sourceIdx: number }[]; // 2000-2119

  // Selection phase actions (context-dependent, from PendingSelection.valid_indices)
  canSelectHandCard: number[];       // indices 0-29
  canSelectRevealed: number[];       // indices 30-39
  canSelectOwnSecurity: number[];    // indices 40-49
  canSelectOppSecurity: number[];    // indices 50-59
  canSelectBreeding: boolean;        // index 99
  canSelectOwnField: number[];       // indices 100-111
  canSelectOppField: number[];       // indices 112-123
  canSelectTrash: number[];          // indices 130-179
  canSelectEffectBranch: number[];   // indices 1000-1009
}

function parseActionMask(
  mask: number[],
  handCards: string[],
  phase: GamePhase
): ParsedActions {
  // During selection phases, valid_indices from PendingSelection
  // map directly to action IDs in the mask.
  // During Main/Breeding, use the standard action ranges.
  // See ACTION_SPEC.md for full formulas.
}
```

This parsed structure drives which UI elements are interactive (highlighted, clickable) at any given moment.
