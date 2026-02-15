# UI & API Plan — Digimon Game Simulator

> Updated 2026-02-15. Reflects engine state after merge of main: 981-float tensor,
> 3,651-card database, DNA Digivolution, 15 keyword mechanics, 10 selection phases,
> revealed cards zone, linked option cards, pilot_training.py agent infrastructure,
> and deck_loader.py (TTS/text import, validation, restricted list enforcement).

## Overview

Five main surfaces:

1. **Game UI** — Play interactive games (Human vs Agent, Human vs Human)
2. **Lobby** — Matchmaking, room codes, game creation
3. **Deck Builder** — Create, edit, and manage decks with card search and filtering
4. **Replay Viewer** — Play back recorded Agent vs Agent games
5. **Admin Dashboard** — Manage agents, launch training runs, view metrics

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
| `BattleArea` | 12 `PermanentSlot` components in a row, scrollable if >8. Each slot is a drop zone. | Drop target for play/digivolve from hand |
| `PermanentSlot` | Top card image, DP badge, level badge, suspend tilt (90deg), keyword badges, linked cards, stack depth indicator | Click to open Stack Inspector, click to select attacker/target, drop zone for digivolve |
| `KeywordBadges` | Small colored tags on permanents: Rush, Blocker, Jamming, Piercing, Retaliation, Blitz, Reboot, Collision, Evade, Armor Purge, Barrier, Security Attack +/-X | Display only, tooltip on hover |
| `LinkedCards` | Sideways mini-cards rendered next to the permanent (for [TS] option cards) | Click to view in Stack Inspector |
| `HandZone` | Horizontally fanned cards with CDN images, dynamic spacing, compresses beyond 7 | Drag card to field (play/digivolve), click for fallback actions |
| `StackInspector` | Right panel: full card image, effect text, DP breakdown, keyword tags, digivolution stack thumbnails, linked cards | Click source thumbnails to view full detail |
| `CardTooltip` | Floating large card image on hover (any card, any zone) | Follows cursor, auto-positions to avoid edges |
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

### 1.3 Interaction Flow — Drag-and-Drop + Click

The primary interaction is **drag-and-drop**. Players drag cards from hand onto valid field zones. Click-to-act is the fallback for actions that don't map to drag (attacking, passing, effect activation). Both input methods coexist — the user can always click instead of dragging.

**Library:** `@dnd-kit/core` + `@dnd-kit/utilities` for drag-and-drop. It supports both mouse and touch, has accessible keyboard fallback, and provides smooth drag overlays.

#### Drag Sources and Drop Targets

| Drag Source | Drop Target | Action Produced | Validation |
|-------------|-------------|-----------------|------------|
| Hand card (Digimon/Tamer) | Empty battle area slot | **Play** (action 0-29) | action mask bit for that hand index |
| Hand card (Option) | Battle area (anywhere) | **Play Option** (action 0-29) | action mask bit + option color req |
| Hand card (Digimon) | Occupied permanent slot | **Digivolve** (action 400-999) | action mask bit for hand*15+field |
| Hand card (DNA) | Occupied permanent slot | **DNA Digivolve** (action 63-92) | then enters SelectMaterial flow |
| Breeding area permanent | Empty battle area slot | **Move from Breeding** (action 61) | action mask bit 61 |

When a card is picked up, **valid drop targets glow green** and invalid zones dim. If the card is dropped on an invalid zone, it snaps back to hand.

#### Drag Overlay

While dragging, a semi-transparent copy of the card follows the cursor (`DragOverlay` from @dnd-kit). The original card in hand dims to 30% opacity. On drop, a brief slide animation transitions the card to its new position.

#### Digivolve vs Play Disambiguation

When dragging a hand card over an occupied field slot, the slot shows **two drop zones stacked vertically**:
- **Top half**: "Digivolve onto [CardName]" (if digivolution is valid for this pair)
- **Bottom half**: "Play to field" (if an empty adjacent slot exists)

If only one action is valid, the entire slot is the drop target.

#### Click-to-Act (Fallback & Non-Drag Actions)

Some actions don't map to drag-and-drop:

**Attacking (click-to-click):**
1. Player clicks an unsuspended permanent in their battle area → it highlights as "attacker"
2. Valid targets light up (opponent permanents + security icon) with red outlines
3. Player clicks a target → sends attack action (100 + attacker*15 + target)
4. **BlockTiming**: If opponent has blockers, UI shows "Block?" prompt with valid blockers highlighted, plus "Decline" button
5. **CounterTiming**: If opponent can blast digivolve, UI shows blast digivolve options, plus "Decline" button
6. Backend resolves combat, returns new state

**DNA Digivolving (drag + click hybrid):**
1. Player drags DNA-capable hand card onto a valid field permanent → sends action (63 + hand_index)
2. Game enters `SelectMaterial` phase — valid first materials highlight on field
3. Player clicks first material
4. Valid second materials highlight
5. Player clicks second material → backend resolves DNA digivolution

**Effect Activation (click):**
1. Player clicks a permanent with activatable effects → effect panel appears
2. Panel shows available effects with descriptions
3. Player clicks "Activate" on one → sends action (1000 + slot*10 + effectIdx)

**Hatching (click):**
- Click egg deck during Breeding phase → sends hatch action (60)

**Passing (click):**
- "Pass" button always visible during player's turn → sends action 62
- During selection phases with `is_optional`, "Decline" button → sends action 62

### 1.4 Card Detail & Stack Inspector

Clicking a permanent on the field opens a **Stack Inspector** panel (replaces the simple card detail sidebar). This is essential for understanding board state in a game with digivolution stacks.

#### Stack Inspector Layout

```
┌─ Stack Inspector ──────────────────────────┐
│                                            │
│  ┌────────────┐  Greymon (BT14-010)        │
│  │            │  Level 4 | Champion        │
│  │  [CARD     │  DP: 8000  (base 6000)     │
│  │   IMAGE]   │  Color: Red                │
│  │            │  Cost: 5                   │
│  │            │  Type: Dinosaur            │
│  └────────────┘                            │
│                                            │
│  Keywords: [Rush] [Piercing]               │
│  SA: +1  |  OPT: 1 available              │
│                                            │
│  ── Card Effect ──────────────────────     │
│  [When Digivolving] Delete 1 of your       │
│  opponent's Digimon with 4000 DP or less.  │
│                                            │
│  ── Inherited Effect ─────────────────     │
│  [Your Turn] This Digimon gets +2000 DP.   │
│                                            │
│  ── Digivolution Stack (2 cards) ────      │
│                                            │
│  ┌──────┐ ┌──────┐                        │
│  │Korom │ │Agumon│   ← click to inspect    │
│  │ L2   │ │ L3   │     individual card     │
│  │      │ │+2000 │     (shows effect text) │
│  └──────┘ └──────┘                        │
│                                            │
│  ── Linked Cards ─────────────────────     │
│  ┌──────┐                                  │
│  │Option│  Training Memory Boost           │
│  │ Card │  (linked sideways)               │
│  └──────┘                                  │
│                                            │
│  [Close]                                   │
└────────────────────────────────────────────┘
```

#### Inspector Features

- **Top card**: full card image + all metadata (name, level, DP, color, cost, type/attribute/form)
- **DP breakdown**: shows base DP + modifier contributions (e.g. "6000 + 2000 inherited = 8000")
- **Keywords**: rendered as colored tags with tooltips explaining each keyword's rule
- **Card effect text**: the main effect, inherited effect, and security effect (if any), each in their own section
- **Digivolution stack**: thumbnail row of all source cards bottom-to-top. Each shows:
  - Card image (small)
  - Name and level
  - DP contribution this source provides (from `SourceInfo.dpContribution`)
  - OPT state indicator (green dot = available, gray = exhausted, no dot = no OPT)
  - Click a source thumbnail to view its full card detail (image + effect text)
- **Linked cards**: any option cards attached sideways, shown as thumbnails with name/effect
- **Close button** or click outside to dismiss

#### Hover vs Click

- **Hover** over any card (hand, field, trash, security) → shows a **quick tooltip** with card image only (like a magnifying glass). Fast and non-blocking.
- **Click** a field permanent → opens the **full Stack Inspector** panel. Persistent until closed.
- **Right-click** a card → future: context menu for advanced actions

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

## 3. Deck Builder

### 3.1 Overview

A full deck editor where players create, save, edit, and manage their decks. Decks are stored server-side (per-user, or per-browser via local storage for anonymous users). The deck builder integrates with the lobby — when starting a game, the player selects from their saved decks.

Digimon TCG deck rules:
- **Main deck**: exactly 50 cards (no more, no less)
- **Egg deck (Digitama)**: 0-5 Digi-Egg cards (separate from main deck)
- **Max 4 copies** of any card (by card number, e.g. max 4x BT14-010) — some cards have lower limits via `max_count_in_deck`
- **No color restrictions** on deck composition
- **Restricted list** (enforced by `deck_loader.RESTRICTED_LIST`):
  - **3 Banned cards** (0 copies): BT2-090 (Matt Ishida), BT5-109 (Mega Digimon Fusion!), EX5-065 (Sayo & Koh)
  - **47 Restricted cards** (max 1 copy): e.g. BT14-002 (Bukamon), EX1-068 (Ice Wall!), P-123 (Ukkomon)
  - **2 Choice groups** — cards from group A and group B cannot coexist in the same deck:
    - Choice 1: EX2-007 (Mother D-Reaper) vs EX7-064 (Shoto Kazama)
    - Choice 2: BT20-037 (Chaosmon: Valdur Arm) vs BT17-035 (Taomon) + EX8-037 (Sakuyamon X-Antibody)

### 3.2 Deck Builder Layout

```
┌──────────────────────────────────────────────────────────────────────────┐
│  DECK BUILDER                                    [My Decks ▼] [+ New]  │
│  Deck: "Red Greymon Aggro"                       [Save] [Export] [Del] │
│                                                                         │
│  ┌─ CARD SEARCH ─────────────────────┐  ┌─ DECK LIST ────────────────┐ │
│  │                                   │  │                            │ │
│  │  Search: [________________] [🔍]  │  │  Main Deck (48/50)         │ │
│  │                                   │  │                            │ │
│  │  ┌─ Filters ────────────────────┐ │  │  Lv.3 Rookies (12)        │ │
│  │  │ Color: [All▼] Kind: [All▼]  │ │  │  ┌────┐┌────┐┌────┐      │ │
│  │  │ Level: [All▼] Set:  [All▼]  │ │  │  │Agum││Agum││Agum│ ...  │ │
│  │  │ Cost:  [0-20] DP:   [All▼]  │ │  │  │x4  ││x4  ││x4  │      │ │
│  │  │ Form:  [All▼] Rarity:[All▼] │ │  │  └────┘└────┘└────┘      │ │
│  │  └──────────────────────────────┘ │  │                            │ │
│  │                                   │  │  Lv.4 Champions (10)       │ │
│  │  Showing 47 results               │  │  ┌────┐┌────┐             │ │
│  │                                   │  │  │Grey││Grey│ ...         │ │
│  │  ┌────┐ ┌────┐ ┌────┐ ┌────┐    │  │  │x4  ││x2  │             │ │
│  │  │Card│ │Card│ │Card│ │Card│    │  │  └────┘└────┘             │ │
│  │  │img │ │img │ │img │ │img │    │  │                            │ │
│  │  │    │ │    │ │    │ │    │    │  │  Lv.5 Ultimates (8)        │ │
│  │  └────┘ └────┘ └────┘ └────┘    │  │  ...                       │ │
│  │  ┌────┐ ┌────┐ ┌────┐ ┌────┐    │  │                            │ │
│  │  │Card│ │Card│ │Card│ │Card│    │  │  Tamers (4)                │ │
│  │  │img │ │img │ │img │ │img │    │  │  ...                       │ │
│  │  │    │ │    │ │    │ │    │    │  │                            │ │
│  │  └────┘ └────┘ └────┘ └────┘    │  │  Options (6)               │ │
│  │                                   │  │  ...                       │ │
│  │  [1] [2] [3] ... [12]  (pages)   │  │                            │ │
│  └───────────────────────────────────┘  │  ── Egg Deck (4/5) ────── │ │
│                                         │  ┌────┐┌────┐             │ │
│  ┌─ CARD DETAIL ───────────────────┐   │  │Koro││Toko│             │ │
│  │  [CARD IMAGE]  BT14-010         │   │  │x2  ││x2  │             │ │
│  │                Greymon          │   │  └────┘└────┘             │ │
│  │  Lv.4 | Champion | Red         │   │                            │ │
│  │  DP: 6000  Cost: 5             │   │  ── Deck Stats ──────────  │ │
│  │  Type: Dinosaur                 │   │  Colors: Red (100%)       │ │
│  │                                 │   │  Avg Cost: 4.2            │ │
│  │  Evo: Red Lv.3 for 3           │   │  Level curve: ▁▃▇▅▂      │ │
│  │                                 │   │                            │ │
│  │  [When Digivolving] Delete 1    │   └────────────────────────────┘ │
│  │  of your opponent's Digimon...  │                                   │
│  │                                 │                                   │
│  │  Inherited: [Your Turn] +2000   │                                   │
│  │                                 │                                   │
│  │  [Add to Deck]  (or click card) │                                   │
│  └─────────────────────────────────┘                                   │
└──────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Card Search & Filters

The search panel provides both text search and faceted filters. Filters are applied server-side via `GET /cards`.

#### Text Search

Free-text search matches against:
- Card name (English)
- Card ID (e.g. "BT14-010")
- Effect text (e.g. "SecurityAttack" or "delete")
- Type/trait text (e.g. "Dragon" or "LIBERATOR")

#### Filter Criteria

| Filter | Type | Values |
|--------|------|--------|
| **Color** | Multi-select chips | Red, Blue, Yellow, Green, Black, Purple, White |
| **Card Kind** | Multi-select chips | Digimon, Tamer, Option, Digi-Egg |
| **Level** | Multi-select chips | 2, 3, 4, 5, 6, 7 |
| **Set** | Dropdown with search | 45 sets: BT1-BT24, EX1-EX8, ST1-ST20, P, LM, RB1 |
| **Play Cost** | Range slider | 0-20 |
| **DP** | Range slider / presets | 1000-16000 (step 1000) |
| **Form** | Dropdown | In-Training, Rookie, Champion, Ultimate, Mega, Hybrid, Armor Form, etc. (16 values) |
| **Attribute** | Dropdown | Vaccine, Data, Virus, Free, Variable, Unknown (+ app types) |
| **Rarity** | Multi-select chips | C, U, R, SR, SEC, P |

Filters combine with AND logic. Multiple selections within a filter use OR (e.g. Color=Red OR Blue).

#### Search Results Grid

- Cards displayed in a **responsive grid** of card images (4-6 columns depending on screen width)
- Each card shows: image, name below, and a small count badge if already in deck (e.g. "2/4")
- **Click** a card in the grid → shows full detail in the Card Detail panel below
- **Double-click** a card (or click "Add to Deck") → adds one copy to the deck
- **Pagination**: 24 cards per page, page controls at bottom
- **Sort**: by card ID (default), name, level, cost, DP

### 3.4 Deck List Panel

The right panel shows the current deck contents, organized by card type and level.

#### Card Grouping

Cards are grouped into sections:
1. **Digi-Eggs** (Level 2) — separate egg deck section at bottom
2. **Rookies** (Level 3)
3. **Champions** (Level 4)
4. **Ultimates** (Level 5)
5. **Megas** (Level 6+)
6. **Tamers**
7. **Options**

Each group header shows: group name and card count.

#### Deck Card Display

Each card in the deck shows:
- Small card image thumbnail
- Card name
- Copy count (x1, x2, x3, x4)
- **Click** → highlight in search results + show in detail panel
- **Right-click / long-press** → remove one copy
- **+/-** buttons on hover to adjust count

#### Deck Stats

A small stats box at the bottom of the deck panel:

| Stat | Display |
|------|---------|
| Main deck count | "48/50" with color (red if not 50) |
| Egg deck count | "4/5" |
| Color distribution | Pie chart or bar (% per color) |
| Average play cost | Number |
| Level curve | Sparkline histogram (bars for each level) |
| Unique cards | Count |

### 3.5 Validation & Restricted List UI

The deck builder shows real-time validation feedback using `deck_loader.validate_deck()` via the `POST /deck/validate` endpoint. Validation runs on every card add/remove.

#### Validation Error Display

```
┌─ VALIDATION ──────────────────────────────┐
│  ❌ Main deck must be exactly 50 (got 48) │
│  ❌ BT14-002 (Bukamon): 2 copies exceeds  │
│     restricted limit of 1                  │
│  ⚠️ Unknown card: FAKE-001 (not in DB)     │
│  ❌ Choice restriction: cannot include     │
│     [EX2-007] and [EX7-064] together       │
└───────────────────────────────────────────┘
```

#### Restricted Card Indicators

- Cards on the **banned** list (limit 0) show a red "BANNED" badge in search results — cannot be added to deck
- Cards on the **restricted** list (limit 1) show an orange "1x" badge in search results — "Add" disabled after 1 copy
- Cards in a **choice group** show a link icon — when one side is in deck, the other side's cards are dimmed with "CONFLICT" tooltip explaining the choice restriction
- The deck list panel marks restricted cards with an orange border and banned cards with a red border

#### Save Gate

- Decks can be saved regardless of validation state (to allow work-in-progress saving)
- Starting a game with an invalid deck shows a confirmation dialog: "This deck has validation errors. Start anyway?"
- Tournament-mode games (future) will enforce strict validation

### 3.6 Deck Management

#### Saved Decks

```
┌─ My Decks ───────────────────────────────┐
│                                          │
│  Red Greymon Aggro        50 cards  [✏️]  │
│  Blue Control             50 cards  [✏️]  │
│  Purple Lilith OTK        48 cards  [✏️]  │
│  Yellow Hybrid             50 cards  [✏️]  │
│                                          │
│  [+ New Deck]                            │
└──────────────────────────────────────────┘
```

#### Import / Export

Backend support: `deck_loader.parse_deck()` auto-detects TTS or text format. The `POST /deck/parse` endpoint returns parsed card IDs + summary, and `POST /deck/validate` adds rule + restricted list checking.

- **Export** → copies deck list to clipboard in digimoncard.io text format:
  ```
  // DigimonCard.io Deck List
  4 Medusamon BT24-017
  2 Agumon BT21-007
  4 Styracomon BT24-018
  ...
  ```
- **Import** → paste or upload, supports two formats:
  - **digimoncard.io text format**: `{count} {name} {card_id}` per line, `//` comments for deck name
  - **TTS (Tabletop Simulator)**: JSON array `["BT24-017", "BT24-017", ...]` (non-card entries like export headers are filtered out)
- After import, automatically validate via `POST /deck/validate` and show any errors/warnings inline
- Invalid cards (not in database) show as warnings, not hard errors — allows importing decks with cards from sets not yet ingested

### 3.7 Drag-and-Drop in Deck Builder

The deck builder also uses @dnd-kit for adding/removing/reordering cards:

| Drag Source | Drop Target | Action |
|-------------|-------------|--------|
| Search result card | Deck list panel | Add 1 copy to deck |
| Deck list card | Outside deck panel / trash icon | Remove 1 copy |

Visual feedback: dragging a card from search shows a ghost card following cursor; deck panel highlights as valid drop target with a green border and "Drop to add" text.

### 3.8 Deck Builder State (Zustand)

```typescript
// stores/deckBuilderStore.ts
interface DeckBuilderStore {
  // Current deck being edited
  currentDeck: DeckData | null;
  isDirty: boolean;  // unsaved changes

  // Card search
  searchQuery: string;
  filters: CardFilters;
  searchResults: CardSearchResult[];
  searchPage: number;
  searchTotal: number;
  sortBy: 'card_id' | 'name' | 'level' | 'cost' | 'dp';

  // Selected card (for detail panel)
  selectedCardId: string | null;

  // Validation (from POST /deck/validate via deck_loader.validate_deck)
  validationResult: DeckValidationResult | null;
  isValidating: boolean;

  // Saved decks list
  savedDecks: DeckSummary[];

  // Actions
  setSearchQuery: (q: string) => void;
  setFilter: (key: keyof CardFilters, value: any) => void;
  clearFilters: () => void;
  setPage: (page: number) => void;
  setSortBy: (sort: string) => void;
  selectCard: (cardId: string) => void;

  addCardToDeck: (cardId: string) => void;
  removeCardFromDeck: (cardId: string) => void;
  setCardCount: (cardId: string, count: number) => void;

  newDeck: (name: string) => void;
  saveDeck: () => Promise<void>;
  loadDeck: (deckId: string) => Promise<void>;
  deleteDeck: (deckId: string) => Promise<void>;
  importDeck: (text: string) => Promise<void>;  // calls POST /deck/parse then validates
  exportDeck: () => string;
  validateDeck: () => Promise<void>;  // calls POST /deck/validate
}

interface DeckValidationResult {
  is_valid: boolean;
  errors: string[];    // hard failures (banned card, wrong size, copy limit)
  warnings: string[];  // soft issues (unknown card IDs)
}

interface DeckData {
  deck_id: string;
  name: string;
  main_deck: { card_id: string; count: number }[];
  egg_deck: { card_id: string; count: number }[];
  created_at: string;
  updated_at: string;
}

interface CardFilters {
  colors: number[];        // CardColor enum values
  kinds: number[];         // CardKind enum values
  levels: number[];
  sets: string[];          // "BT14", "ST1", etc.
  costMin: number | null;
  costMax: number | null;
  dpMin: number | null;
  dpMax: number | null;
  forms: string[];
  attributes: string[];
  rarities: number[];
}

interface CardSearchResult {
  card_id: string;
  card_name: string;
  level: number | null;
  cost: number;
  dp: number | null;
  colors: number[];
  kind: number;
  in_deck_count: number;  // 0-4, how many already in current deck
}
```

---

## 4. Replay Viewer — Agent vs Agent Playback

### 4.1 Concept

Record full game state snapshots at every action during Agent vs Agent games. The replay viewer loads the recording and lets the user scrub through it like a video timeline.

### 4.2 Recording Format

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

### 4.3 Replay UI

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

### 4.4 Replay State Store

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

## 5. Admin Dashboard

### 5.1 Pages

| Page | Purpose |
|------|---------|
| **Agent List** | View all trained agents, their type, training status, win rates |
| **Agent Detail** | View agent config, training history, play sample games |
| **Training Jobs** | Launch new training runs, monitor progress, stop/resume |
| **Matchup Matrix** | Run Agent A vs Agent B simulations, see win rates in a grid |
| **Replay Browser** | List and filter recorded replays by agents, date, winner |
| **Card Database** | Browse the 3,651-card database with set/color/kind filters |

### 5.2 Agent Management

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

### 5.3 Training Job Panel

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

### 5.4 Matchup Matrix

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

## 6. API Endpoints

### 6.1 Existing Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/` | Health check |
| POST | `/simulate` | Batch simulation (now accepts TTS/text deck strings via `parse_deck()`) |
| POST | `/game/create` | Create game session (accepts `deck1_raw`/`deck2_raw` for TTS/text import) |
| POST | `/game/{id}/action` | Execute action |
| POST | `/game/{id}/step` | Advance interactive game |
| GET | `/game/{id}/state` | Get game state |
| GET | `/game/{id}/mask` | Get action mask |
| GET | `/game/{id}/log` | Get game log |
| DELETE | `/game/{id}` | Delete session |
| POST | `/deck/parse` | Parse deck string (TTS or text) → card IDs + summary (**new**) |
| POST | `/deck/validate` | Parse + validate deck against rules & restricted list (**new**) |

The `/deck/parse` and `/deck/validate` endpoints use `deck_loader.py` (`parse_deck()`, `validate_deck()`, `summarize_deck()`). Both accept a `DeckRequest` body with a `deck` string field.

**`POST /deck/validate` response:**
```json
{
  "is_valid": false,
  "errors": [
    "Main deck must be exactly 50 cards (got 48)",
    "BT14-002 (Bukamon): 2 copies exceeds restricted limit of 1",
    "BT2-090 (Matt Ishida) is banned",
    "Choice restriction violated: cannot include cards from [EX2-007] and [EX7-064] in the same deck"
  ],
  "warnings": ["Unknown card ID: FAKE-999 (not in card database)"],
  "summary": {"BT24-017": 4, "BT24-018": 4, "...": "..."},
  "total_cards": 48
}
```

### 6.2 New Endpoints — Game UI

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

### 6.3 New Endpoints — Lobby & Multiplayer

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

### 6.4 New Endpoints — Deck Builder

#### `GET /cards` (enhanced)

Card search with full filtering. All parameters are optional.

```
GET /cards?q=Greymon&color=0&kind=0&level=4&set=BT14&cost_min=3&cost_max=7&form=Champion&rarity=2&sort=card_id&page=1&limit=24
```

| Param | Type | Description |
|-------|------|-------------|
| `q` | string | Text search (name, ID, effect text, type) |
| `color` | int[] | CardColor enum values (comma-separated, OR logic) |
| `kind` | int[] | CardKind enum values |
| `level` | int[] | Level values |
| `set` | string[] | Set prefixes (e.g. "BT14,ST1") |
| `cost_min` / `cost_max` | int | Play cost range |
| `dp_min` / `dp_max` | int | DP range |
| `form` | string[] | Form names (e.g. "Champion,Mega") |
| `attribute` | string[] | Attribute names |
| `rarity` | int[] | Rarity enum values |
| `sort` | string | Sort field: card_id, name, level, cost, dp |
| `page` | int | Page number (1-based) |
| `limit` | int | Results per page (default 24, max 100) |

```json
// Response
{
  "cards": [
    {
      "card_id": "BT14-010",
      "card_name_eng": "Greymon",
      "level": 4,
      "play_cost": 5,
      "dp": 6000,
      "card_colors": [0],
      "card_kind": 0,
      "rarity": 1,
      "form_eng": ["Champion"],
      "type_eng": ["Dinosaur"],
      "attribute_eng": ["Vaccine"],
      "effect_description_eng": "[When Digivolving] Delete 1 of your opponent's Digimon with 4000 DP or less.",
      "inherited_effect_description_eng": "[Your Turn] This Digimon gets +2000 DP.",
      "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 3}]
    }
  ],
  "total": 47,
  "page": 1,
  "page_size": 24
}
```

#### `GET /decks`

List saved decks for the current user/session.

```json
{
  "decks": [
    {
      "deck_id": "uuid",
      "name": "Red Greymon Aggro",
      "main_deck_count": 50,
      "egg_deck_count": 4,
      "colors": [0],
      "updated_at": "2026-02-15T..."
    }
  ]
}
```

#### `POST /decks`

Create a new deck.

```json
// Request
{
  "name": "Red Greymon Aggro",
  "main_deck": [
    {"card_id": "BT14-010", "count": 4},
    {"card_id": "BT14-005", "count": 4}
  ],
  "egg_deck": [
    {"card_id": "BT14-001", "count": 4}
  ]
}

// Response
{
  "deck_id": "uuid",
  "name": "Red Greymon Aggro",
  "created_at": "2026-02-15T..."
}
```

#### `GET /decks/{deck_id}`

Get full deck contents.

#### `PUT /decks/{deck_id}`

Update a deck (name and/or card list).

#### `DELETE /decks/{deck_id}`

Delete a deck.

#### `POST /decks/{deck_id}/validate`

Validate a saved deck against Digimon TCG rules and restricted list. Uses `deck_loader.validate_deck()` internally — same validation as `POST /deck/validate` but operates on a saved deck by ID rather than raw text. Returns `DeckValidationResult`:

```json
// Response
{
  "is_valid": false,
  "errors": [
    "Main deck must be exactly 50 cards (got 48)",
    "BT14-010 (Greymon): 5 copies exceeds max 4 per deck",
    "BT2-090 (Matt Ishida) is banned",
    "BT14-002 (Bukamon): 2 copies exceeds restricted limit of 1",
    "Choice restriction violated: cannot include cards from [EX2-007] and [EX7-064] in the same deck"
  ],
  "warnings": [
    "Unknown card ID: FAKE-001 (not in card database)"
  ],
  "summary": {"BT14-010": 5, "BT14-005": 4, "...": "..."},
  "total_cards": 48
}
```

### 6.5 New Endpoints — Replay System

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

### 6.6 New Endpoints — Admin / Agent Management

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

## 7. Frontend Architecture

### 7.1 Directory Structure

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
│   │   ├── cardApi.ts           # Card search with filtering and pagination
│   │   ├── deckApi.ts           # Deck CRUD (create, save, load, delete, validate)
│   │   ├── replayApi.ts         # Replay CRUD
│   │   ├── agentApi.ts          # Agent management
│   │   └── trainingApi.ts       # Training jobs
│   ├── ws/
│   │   ├── gameSocket.ts        # WebSocket connection for game
│   │   └── lobbySocket.ts       # WebSocket connection for lobby
│   ├── stores/
│   │   ├── gameStore.ts         # Interactive game state (Zustand)
│   │   ├── lobbyStore.ts        # Room list, queue status, current room
│   │   ├── deckBuilderStore.ts  # Deck editor, card search, filters, saved decks
│   │   ├── replayStore.ts       # Replay playback state
│   │   ├── uiStore.ts           # UI state (selected cards, modals)
│   │   └── adminStore.ts        # Agent/training state
│   ├── pages/
│   │   ├── HomePage.tsx         # Landing: new game / play online / decks / replays / admin
│   │   ├── LobbyPage.tsx        # Room creation, join, quickmatch, active rooms
│   │   ├── WaitingRoomPage.tsx  # Pre-game room (deck select, ready up)
│   │   ├── DeckBuilderPage.tsx  # Full deck builder with search + editor
│   │   ├── DeckListPage.tsx     # Browse saved decks
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
│   │   ├── deckbuilder/
│   │   │   ├── CardSearchPanel.tsx  # Text search + filter controls + result grid
│   │   │   ├── FilterBar.tsx        # Color/kind/level/set/cost/dp/form/rarity filters
│   │   │   ├── CardGrid.tsx         # Paginated card image grid
│   │   │   ├── DeckListPanel.tsx    # Right panel: deck contents grouped by level
│   │   │   ├── DeckCardEntry.tsx    # Single card row in deck list (thumbnail + count)
│   │   │   ├── DeckStats.tsx        # Deck statistics (color dist, cost curve, counts)
│   │   │   ├── DeckSelector.tsx     # Dropdown to switch between saved decks
│   │   │   └── ImportExport.tsx     # Import/export deck list text
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
│   │       ├── Card.tsx         # Card renderer (image + overlays + drag source)
│   │       ├── CardStack.tsx    # Digivolution stack view
│   │       ├── StackInspector.tsx # Full card detail + stack browser panel
│   │       ├── CardTooltip.tsx  # Hover tooltip (large card image)
│   │       └── Modal.tsx
│   ├── hooks/
│   │   ├── useGameSocket.ts    # WebSocket hook for game
│   │   ├── useLobbySocket.ts   # WebSocket hook for lobby
│   │   ├── useActionMask.ts    # Parse mask into actionable UI state
│   │   ├── useCardImage.ts     # CDN image loading + fallback
│   │   ├── useDragDrop.ts      # Drag validation + action resolution
│   │   └── useReplayPlayer.ts  # Playback timer logic
│   ├── utils/
│   │   ├── actionDecoder.ts    # Decode action ID -> human description
│   │   └── constants.ts        # Action ranges, phase names, keyword list
│   └── types/
│       ├── game.ts             # GameState, PlayerState, PermanentInfo
│       ├── replay.ts           # ReplayData, Frame
│       └── admin.ts            # Agent, TrainingJob, Matchup
```

### 7.2 Key Libraries

| Library | Purpose |
|---------|---------|
| React 19 | UI framework |
| TypeScript | Type safety |
| Vite | Build tool |
| Zustand | State management (simpler than Redux for this scope) |
| React Router | Page navigation |
| Axios | REST API calls |
| native WebSocket | Real-time game updates (no library needed) |
| `@dnd-kit/core` | Drag-and-drop (accessible, touch support, smooth overlays) |
| Recharts or Chart.js | Training metrics charts |
| Tailwind CSS | Styling (utility-first, fast iteration) |

### 7.3 Card Image Pipeline

Card images are sourced from the **digimoncard.io CDN** using a deterministic URL pattern:

```
https://images.digimoncard.io/images/cards/{CARD_ID}.webp
```

Examples:
- `BT14-001` → `https://images.digimoncard.io/images/cards/BT14-001.webp`
- `ST1-01` → `https://images.digimoncard.io/images/cards/ST1-01.webp`

No image URLs need to be stored in `cards.json` — they are constructed at render time from the card ID.

#### Image Loading Strategy

```
┌─ Image Resolution Order ──────────────────────────┐
│                                                    │
│  1. Check browser cache (Service Worker / HTTP)    │
│  2. Load from CDN: images.digimoncard.io           │
│  3. On error → show placeholder                    │
│                                                    │
└────────────────────────────────────────────────────┘
```

**`useCardImage(cardId)` hook:**

```typescript
function useCardImage(cardId: string): {
  src: string;
  isLoading: boolean;
  hasError: boolean;
} {
  const cdnUrl = `https://images.digimoncard.io/images/cards/${cardId}.webp`;
  // Uses <img onLoad/onError> to track state
  // Returns cdnUrl as src, tracks loading/error
  // On error, Card component renders placeholder fallback
}
```

#### Card Component Rendering

The `Card` component renders differently depending on context:

| Context | Size | Shows | Image |
|---------|------|-------|-------|
| **Hand** (full) | ~100x140px | Full card image | CDN image |
| **Field** (permanent top) | ~80x112px | Card image + DP/level badge overlay | CDN image |
| **Stack thumbnail** (inspector) | ~60x84px | Small card image + level badge | CDN image |
| **Hover tooltip** | ~200x280px | Large card image only | CDN image |
| **Face-down** (opponent hand, security) | ~80x112px | Card back image | Local asset |
| **Placeholder fallback** | Same as context | Colored rectangle + name/level/DP text | None |

```typescript
interface CardProps {
  cardId: string;
  size: 'sm' | 'md' | 'lg' | 'xl';  // 60, 80, 100, 200px wide
  faceDown?: boolean;
  suspended?: boolean;     // rotates 90deg clockwise
  dimmed?: boolean;        // 30% opacity (during drag)
  highlighted?: boolean;   // green glow (valid drop target)
  targeted?: boolean;      // red glow (attack target)
  overlay?: {
    dp?: number;
    level?: number;
    keywords?: string[];
    saModifier?: number;
  };
  onClick?: () => void;
  onHover?: () => void;
  draggable?: boolean;     // enables @dnd-kit drag source
}
```

#### Placeholder Fallback

When CDN image fails to load (or during development), render a styled placeholder:

```
┌──────────────┐
│  ▓▓▓▓▓▓▓▓▓▓  │  ← Color bar (Red/Blue/Yellow/Green/Purple/Black/White)
│              │
│   Greymon    │  ← Card name
│              │
│   Lv.4      │  ← Level
│   DP: 6000  │  ← DP
│   Cost: 5   │  ← Play cost
│              │
│  [Champion]  │  ← Form
└──────────────┘
```

Colors map to `CardColor` enum: Red=#D32F2F, Blue=#1976D2, Yellow=#FBC02D, Green=#388E3C, Purple=#7B1FA2, Black=#424242, White=#BDBDBD.

#### Caching & Performance

- **Browser caching**: CDN images have long cache headers; once loaded, they're cached for the session
- **Lazy loading**: Cards not in viewport use `loading="lazy"` on `<img>` tags
- **Preloading**: When a game starts, preload images for all cards in both decks (50-55 unique cards per game) in a background `Promise.all`
- **Card back**: Single static asset in `/public/card-back.webp`, loaded once

#### Backend Card Image Proxy (Optional, Phase 6)

If CDN direct access has CORS issues or for offline/self-hosted deployments, add a backend proxy:

```
GET /cards/{card_id}/image
→ Proxies to https://images.digimoncard.io/images/cards/{card_id}.webp
→ Adds CORS headers
→ Caches in /data/card_images/ (file-based)
→ Returns image/webp
```

This is optional — direct CDN access should work for most cases since image CDNs typically allow cross-origin requests.

### 7.4 Drag-and-Drop Architecture

#### DnD Provider Setup

The entire `GameBoard` is wrapped in a `DndContext` from @dnd-kit:

```typescript
<DndContext
  onDragStart={handleDragStart}
  onDragEnd={handleDragEnd}
  modifiers={[restrictToWindowEdges]}
>
  <GameBoard />
  <DragOverlay>
    {activeDragCard && <Card cardId={activeDragCard} size="md" />}
  </DragOverlay>
</DndContext>
```

#### Drag Data

Each draggable card carries its hand index and card ID:

```typescript
// Hand card draggable
useDraggable({
  id: `hand-${handIndex}`,
  data: { type: 'hand-card', handIndex, cardId }
})
```

#### Drop Zones

```typescript
// Empty battle area slot
useDroppable({
  id: `field-slot-${slotIndex}`,
  data: { type: 'empty-field-slot', slotIndex }
})

// Occupied permanent (digivolve target)
useDroppable({
  id: `permanent-${slotIndex}`,
  data: { type: 'occupied-field-slot', slotIndex, permanentId }
})

// Battle area general (for options)
useDroppable({
  id: 'battle-area',
  data: { type: 'battle-area' }
})
```

#### Drop Validation & Action Resolution

```typescript
function handleDragEnd(event: DragEndEvent) {
  const { active, over } = event;
  if (!over) return; // dropped outside a target

  const handIndex = active.data.current?.handIndex;
  const target = over.data.current;

  if (target.type === 'empty-field-slot') {
    // Play card: action = handIndex (0-29)
    if (actionMask[handIndex]) sendAction(handIndex);
  }
  else if (target.type === 'occupied-field-slot') {
    // Digivolve: action = 400 + handIndex * 15 + target.slotIndex
    const action = 400 + handIndex * 15 + target.slotIndex;
    if (actionMask[action]) sendAction(action);
  }
  else if (target.type === 'battle-area') {
    // Play option/tamer: action = handIndex
    if (actionMask[handIndex]) sendAction(handIndex);
  }
}
```

#### Visual Feedback During Drag

| State | Visual |
|-------|--------|
| Card picked up | Original dims to 30% opacity; `DragOverlay` shows card following cursor |
| Over valid target | Drop zone glows green with dashed border |
| Over invalid target | Drop zone shows red X or no highlight |
| Over digivolve target | Permanent slot shows "Digivolve" label overlay |
| Dropped successfully | Brief slide animation to final position |
| Dropped on invalid | Card snaps back to hand position |

#### Mobile/Touch Support

@dnd-kit includes touch sensors by default. Additional config:

```typescript
const sensors = useSensors(
  useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
  useSensor(TouchSensor, { activationConstraint: { delay: 150, tolerance: 5 } }),
  useSensor(KeyboardSensor)  // accessibility
);
```

The `delay: 150` on touch prevents accidental drags when scrolling.

---

## 8. Implementation Phases

### Phase 1: Foundation
- Set up React + Vite + TypeScript project in `frontend/`
- Add CORS middleware to FastAPI
- Implement `GET /cards` endpoint with pagination (3,651 cards)
- Build `Card` component with CDN image loading (`useCardImage` hook) + placeholder fallback
- Build card back asset for face-down cards
- Build `GameBoard` layout with all zones (static/mock data)
- Build `MemoryGauge`, `PhaseIndicator`, `GameLog` components
- Set up @dnd-kit `DndContext` with `DragOverlay` around `GameBoard`
- Build `StackInspector` panel (click permanent → full card detail + digivolution stack)
- Build `CardTooltip` (hover → large card image)
- Reference `TENSOR_SPEC.md` and `ACTION_SPEC.md` for all mappings

### Phase 2: Deck Builder
- ~~Implement deck parsing and validation~~ (**Done**: `deck_loader.py` — `parse_deck()`, `validate_deck()`, `RESTRICTED_LIST`)
- ~~Implement `POST /deck/parse` and `POST /deck/validate`~~ (**Done**: already in `api.py`)
- Enhance `GET /cards` with full filter support (color, kind, level, set, cost, DP, form, attribute, rarity, text search)
- Add `CardDatabase.search()` method with query building
- Implement `/decks` CRUD endpoints with file-based storage (`decks.py`, builds on `deck_loader.py`)
- Implement `POST /decks/{id}/validate` wrapping existing `validate_deck()`
- Build `CardSearchPanel` with `FilterBar` (multi-select chips, range sliders, dropdowns)
- Build `CardGrid` with paginated card images and "in deck" count badges
- Build `DeckListPanel` with level-grouped cards, count controls, and drag-to-add
- Build `DeckStats` (color distribution, cost curve, card counts)
- Build `ValidationPanel` showing errors/warnings from `validate_deck()` with restricted list indicators
- Build import/export using `POST /deck/parse` (supports TTS + digimoncard.io text format)
- Build `DeckSelector` dropdown for switching between saved decks
- Integrate deck selection into lobby (pick from saved decks when creating/joining games)

### Phase 3: Interactive Game (Human vs Agent)
- Implement WebSocket endpoint `/ws/game/{game_id}`
- Build `useGameSocket` hook
- Implement drag-and-drop: hand → field (play), hand → permanent (digivolve)
- Build drop zone validation with action mask checking
- Build visual drag feedback (green glow valid, dim invalid, digivolve label)
- Implement click-to-act fallback for attacking, effects, passing
- Build `ActionBar` with contextual buttons derived from action mask
- Build `useActionMask` hook (translates 2120 mask -> UI-friendly action list)
- Build all 10 selection phase UIs (SelectionOverlay, TrashBrowser, StackBrowser, EffectChoicePanel, SecurityBrowser, DnaDigivolveFlow)
- Implement keyword badge rendering on permanents
- Implement linked card display
- Implement revealed cards zone
- Preload card images for both decks on game start
- Connect `GamePage` end-to-end: create game -> play -> game over

### Phase 4: Multiplayer (PvP)
- Implement `/rooms` CRUD endpoints and room code generation
- Implement `/ws/lobby` WebSocket for real-time room updates
- Add `to_player_json(player_id)` to `Game` for player-specific state filtering
- Build `LobbyPage`, `WaitingRoomPage` components
- Extend `/ws/game/{game_id}` for two-player connections
- Implement quickmatch queue with pairing logic
- Add disconnection handling, reconnect tokens, forfeit timeout
- Add spectator mode (read-only WebSocket connection)
- Add rematch flow

### Phase 5: Replay System
- Implement replay recording in backend (wrap HeadlessGame, use `to_ui_json()`)
- Implement `/replays` CRUD endpoints
- Build `PlaybackControls` + `Timeline` components
- Build `ReplayPage` reusing `GameBoard` in read-only mode
- Build `ReplayListPage` with filtering

### Phase 6: Admin Dashboard
- Implement `/agents` CRUD endpoints
- Implement `/training/start` wrapping `pilot_training.py` as subprocess
- Expose `WinRateCallback` metrics via `/training/jobs/{id}/metrics`
- Build agent list and detail views
- Build training job launch form
- Build metrics chart (reward curve + win rate over timesteps)
- Implement `/matchup` endpoints
- Build matchup matrix view

### Phase 7: Polish
- Backend image proxy (`GET /cards/{id}/image`) for CORS/offline fallback
- Attack arrows (SVG overlay with pulsing animation)
- Suspend/unsuspend animations
- Card play slide animation (hand → field)
- Sound effects
- Mobile-responsive layout (touch drag tuning)
- Error handling and reconnection logic

---

## 9. Backend Changes Summary

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

### Already implemented (from recent main merge):

| File | Status | What it provides |
|------|--------|-----------------|
| `digimon_gym/engine/data/deck_loader.py` | **Done** | `parse_tts()`, `parse_text()`, `parse_deck()` (auto-detect), `validate_deck()`, `summarize_deck()`, `expand_deck_dict()`, `RESTRICTED_LIST` (3 banned, 47 restricted, 2 choice groups), `CardRestriction` / `DeckValidationResult` dataclasses |
| `digimon_gym/api.py` (deck endpoints) | **Done** | `POST /deck/parse`, `POST /deck/validate`, updated `POST /game/create` (raw deck strings), updated `POST /simulate` (TTS/text deck strings) |
| `tests/test_deck_loader.py` | **Done** | 486 lines, full coverage of parsing + validation + restricted list + Medusamon integration deck |

### New files still to create:

| File | Purpose |
|------|---------|
| `digimon_gym/api_ws.py` | WebSocket game handler (PvA + PvP + spectator) |
| `digimon_gym/lobby.py` | Room management, quickmatch queue, lobby WebSocket |
| `digimon_gym/replay.py` | Replay recording and storage |
| `digimon_gym/agents/registry.py` | Agent registration (wraps pilot_training.py) |
| `digimon_gym/training/manager.py` | Training job lifecycle (subprocess wrapper) |
| `digimon_gym/decks.py` | Deck CRUD operations (save/load/list/delete), builds on `deck_loader.py` for validation |
| `digimon_gym/training/metrics.py` | Metrics collection from WinRateCallback |

### Modifications to existing files:

| File | Changes |
|------|---------|
| `digimon_gym/api.py` | Add card (paginated), room, deck CRUD, replay, agent, training, matchup endpoints; mount WebSockets; add CORS middleware |
| `digimon_gym/engine/game.py` | Add `to_ui_json()`, `to_player_json(player_id)`, `action_description(action_id)` |
| `digimon_gym/engine/runners/headless_game.py` | Add replay recording hooks |
| `digimon_gym/engine/data/card_database.py` | Add `to_dict()`, `search(set, color, kind, level, cost, dp, form, attribute, rarity, query)` for card API with full filter support |

### Data storage:

For the pre-alpha stage, use **file-based storage** (JSON files):
- `data/decks/` — One JSON file per saved deck (`{deck_id}.json`)
- `data/replays/` — One JSON file per replay
- `data/agents.json` — Agent registry
- `data/training_jobs.json` — Training job history
- `models/` — Saved model files (`.zip` for SB3)

Migrate to SQLite or PostgreSQL when needed.

---

## 10. Action Mask -> UI Mapping

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
