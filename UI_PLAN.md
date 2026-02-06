# UI & API Plan — Digimon Game Simulator

## Overview

Three main surfaces:

1. **Game UI** — Play interactive Human vs Agent games in the browser
2. **Replay Viewer** — Play back recorded Agent vs Agent games
3. **Admin Dashboard** — Manage agents, launch training runs, view metrics

Tech stack: **React 19 + TypeScript + Vite**, with **Zustand** for state management and **WebSocket** for real-time game communication. The existing **FastAPI** backend is extended with new endpoints.

---

## 1. Game UI — Interactive Play

### 1.1 Board Layout

Inspired by the [WE-Kaito simulator](https://github.com/WE-Kaito/digimon-tcg-simulator), the board is a single-screen layout with mirrored player halves. All zones from the Digimon TCG are represented:

```
┌──────────────────────────────────────────────────────────────┐
│  OPPONENT AREA (top half, cards inverted)                    │
│  ┌─────┐ ┌─────────────────────────────────┐ ┌────┐ ┌────┐ │
│  │Egg  │ │  Battle Area (8+ slots)         │ │Deck│ │Sec │ │
│  │Deck │ │  [Perm][Perm][Perm]...          │ │    │ │ury │ │
│  └─────┘ └─────────────────────────────────┘ └────┘ └────┘ │
│  ┌─────┐                                            ┌────┐ │
│  │Breed│                                            │Trsh│ │
│  └─────┘                                            └────┘ │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │            MEMORY GAUGE  [-10 ... 0 ... +10]         │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────┐                                            ┌────┐ │
│  │Breed│                                            │Trsh│ │
│  └─────┘                                            └────┘ │
│  ┌─────┐ ┌─────────────────────────────────┐ ┌────┐ ┌────┐ │
│  │Egg  │ │  Battle Area (8+ slots)         │ │Deck│ │Sec │ │
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
| `PermanentSlot` | Single permanent: top card art, DP badge, level badge, suspend tilt, digivolution stack indicator | Click to select attacker/target, hover for detail |
| `HandZone` | Horizontally fanned cards, dynamic spacing | Click card to see valid actions (play, digivolve targets), drag-and-drop optional |
| `SecurityStack` | Face-down pile with count badge | Hover shows count, click to browse (own only) |
| `DeckPile` | Face-down pile with count badge | — |
| `EggDeck` | Digitama pile with count | Click for hatch action |
| `BreedingArea` | Single permanent slot | Click for move-to-battle action |
| `TrashPile` | Count badge, click to browse | Modal dialog listing all cards |
| `MemoryGauge` | 21-segment horizontal bar, color-coded | Display only (updated by server) |
| `PhaseIndicator` | Shows current phase name + turn number | Display only |
| `CardDetail` | Right sidebar, shows full card image + text when hovering | — |
| `GameLog` | Scrollable text panel showing VerboseLogger output | Auto-scrolls to bottom |
| `ActionBar` | Contextual buttons based on game state | Pass, Hatch, Move from Breeding, confirm attack target |

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
3. Player clicks a target → sends attack action
4. Backend resolves combat, returns new state

**Digivolving:**
1. Player clicks a card in hand that can digivolve
2. Valid digivolution targets on the field highlight
3. Player clicks a target permanent → sends digivolve action

**Hatching / Moving from Breeding:**
- Action bar shows "Hatch" button when in Breeding phase with eggs available
- Action bar shows "Move" button when breeding area has a L3+ digimon

**Passing:**
- "Pass" button always visible during player's turn, sends action 62

### 1.4 Visual Effects (Phase 2)

These are nice-to-have and can be added incrementally:
- Attack arrows (SVG lines between attacker and target)
- Card play animation (hand → field slide)
- Suspend/unsuspend tilt animation (CSS transform rotate 30deg)
- DP modifier badges (green +, red -)
- Security check flip animation
- Turn transition overlay

### 1.5 State Management (Zustand)

```typescript
// stores/gameStore.ts
interface GameStore {
  // Connection
  gameId: string | null;
  wsConnected: boolean;

  // Game state (from server)
  turnCount: number;
  currentPhase: string;
  currentPlayer: 1 | 2;
  memoryGauge: number;
  isGameOver: boolean;
  winner: number | null;
  player1: PlayerState;
  player2: PlayerState;

  // Action mask (from server)
  actionMask: number[];    // 2120 elements

  // Local UI state
  selectedHandCard: number | null;
  selectedAttacker: number | null;
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
  handIds: string[];       // only for "our" player
  securityCount: number;
  deckCount: number;
  battleArea: PermanentInfo[];
  breedingArea: PermanentInfo | null;
  trashCards: string[];    // card IDs
}

interface PermanentInfo {
  topCardId: string;
  topCardName: string;
  dp: number;
  level: number;
  isSuspended: boolean;
  sourceCount: number;
}
```

---

## 2. Replay Viewer — Agent vs Agent Playback

### 2.1 Concept

Record full game state snapshots at every action during Agent vs Agent games. The replay viewer loads the recording and lets the user scrub through it like a video timeline.

### 2.2 Recording Format

Each recorded game is a JSON file:

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
      "state": { /* full to_json() snapshot */ },
      "logs": ["Game started. Player 1 goes first."]
    },
    {
      "frame_id": 1,
      "action_id": 60,
      "action_description": "Player 1: Hatch",
      "player": 1,
      "state": { /* to_json() */ },
      "logs": ["Player 1 hatches ST1-01 Koromon"]
    }
    // ... one frame per action
  ]
}
```

### 2.3 Replay UI

The replay viewer reuses the same `GameBoard` component from the interactive game, but in read-only mode with playback controls:

```
┌────────────────────────────────────────────────────────┐
│  Same board layout as interactive game (read-only)     │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │ ◄◄  ◄  ▶  ►►  │  Frame 47/187  │  1x  2x  4x  │  │
│  │ Timeline scrubber ════════●══════════════════════│  │
│  └──────────────────────────────────────────────────┘  │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Action Log (synced to current frame)            │  │
│  │  > Player 1: Play Agumon (cost 3, memory 4→1)   │  │
│  │  > Player 1: Attack with Greymon → Security      │  │
│  │  > Player 1: Pass turn                           │  │
│  │  ► Player 2: Hatch Tokomon                       │  │
│  └──────────────────────────────────────────────────┘  │
│                                                        │
│  Metadata: Agent1=greedy vs Agent2=ppo_v3 | Winner: P1│
└────────────────────────────────────────────────────────┘
```

**Controls:**
- Play/Pause with configurable speed (1x, 2x, 4x, 0.5x)
- Step forward/backward one frame
- Jump to start/end
- Scrubber bar to seek to any frame
- Auto-scroll log to current frame
- Both players' hands are visible (since it's a replay, no hidden information)

### 2.4 Replay State Store

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

## 3. Admin Dashboard

### 3.1 Pages

| Page | Purpose |
|------|---------|
| **Agent List** | View all trained agents, their type, training status, win rates |
| **Agent Detail** | View agent config, training history, play sample games |
| **Training Jobs** | Launch new training runs, monitor progress, stop/resume |
| **Matchup Matrix** | Run Agent A vs Agent B simulations, see win rates in a grid |
| **Replay Browser** | List and filter recorded replays by agents, date, winner |
| **Card Database** | Browse the 222-card database, view stats |

### 3.2 Agent Management

```
┌─────────────────────────────────────────────────────────┐
│  Agents                                    [+ New Agent] │
│─────────────────────────────────────────────────────────│
│  Name          │ Type        │ Status  │ Win Rate │ Act │
│  greedy_v1     │ Greedy      │ Ready   │ 43.2%    │ ▶ 📊│
│  ppo_v3        │ MaskablePPO │ Ready   │ 61.8%    │ ▶ 📊│
│  ppo_v4        │ MaskablePPO │ Training│ —        │ ⏹ 📊│
│  q_deck_rec_v1 │ Q-DeckRec   │ Ready   │ 55.1%    │ ▶ 📊│
└─────────────────────────────────────────────────────────┘
```

### 3.3 Training Job Panel

```
┌─────────────────────────────────────────────────────────┐
│  New Training Job                                        │
│                                                          │
│  Agent Type:   [MaskablePPO ▼]                          │
│  Base Agent:   [ppo_v3 ▼] (or "from scratch")          │
│  Deck Pool:    [ST1 Starter ▼] [BT14 Meta ▼]           │
│  Timesteps:    [1,000,000    ]                          │
│  Learning Rate:[0.0003       ]                          │
│                                                          │
│  [Launch Training]                                       │
│                                                          │
│  ── Active Jobs ──                                      │
│  ppo_v4 │ 340k/1M steps │ ████████░░░░ 34% │ [Stop]    │
│  Reward curve: [sparkline chart]                         │
└─────────────────────────────────────────────────────────┘
```

### 3.4 Matchup Matrix

```
┌──────────────────────────────────────────────┐
│  Matchup Matrix (100 games each)             │
│                                              │
│              │greedy│ppo_v3│ppo_v4│q_deck_v1│
│  greedy      │  —   │ 38%  │ 41%  │  45%    │
│  ppo_v3      │ 62%  │  —   │ 52%  │  58%    │
│  ppo_v4      │ 59%  │ 48%  │  —   │  54%    │
│  q_deck_v1   │ 55%  │ 42%  │ 46%  │   —     │
│                                              │
│  [Run Full Matrix]  [Export CSV]             │
└──────────────────────────────────────────────┘
```

---

## 4. API Endpoints

### 4.1 Existing Endpoints (keep as-is)

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

### 4.2 New Endpoints — Game UI

#### WebSocket: `/ws/game/{game_id}`

Replace polling with a WebSocket connection for real-time game state updates.

**Client → Server messages:**
```json
{"type": "action", "action_id": 60}
{"type": "step"}
```

**Server → Client messages:**
```json
{
  "type": "state_update",
  "state": { /* to_json() */ },
  "action_mask": [0, 0, 1, ...],
  "logs": ["Player 1 hatches..."],
  "is_human_turn": true,
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

This replaces the current REST polling pattern with push-based updates. The existing REST endpoints remain for non-interactive use (scripts, agents, testing).

#### `GET /cards`

Return the full card database for client-side card rendering.

```json
{
  "cards": {
    "ST1-01": {
      "card_id": "ST1-01",
      "card_name": "Koromon",
      "card_color": "Red",
      "card_kind": "Digi-Egg",
      "level": 2,
      "dp": null,
      "cost": 0,
      "image_url": "...",
      "card_text": "..."
    }
  }
}
```

#### `GET /cards/{card_id}/image`

Return card image. Initially can return a placeholder; later integrate with card image CDN or local assets.

### 4.3 New Endpoints — Replay System

#### `POST /replay/record`

Start recording a game. This creates a headless game that records every frame.

```json
// Request
{
  "deck1": ["ST1-01", ...],
  "deck2": ["BT14-001", ...],
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

```json
// Query params: ?agent1=ppo_v3&agent2=greedy&limit=20&offset=0

// Response
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

```json
// Response — the full recording format from §2.2
{
  "metadata": { ... },
  "frames": [ ... ]
}
```

#### `DELETE /replays/{replay_id}`

Delete a replay.

### 4.4 New Endpoints — Admin / Agent Management

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

Register a new agent (greedy agents are instant; RL agents need training).

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
  "base_agent_id": "ppo_v3"  // optional, for fine-tuning
}
```

#### `GET /agents/{agent_id}`

Get agent details including training history.

#### `DELETE /agents/{agent_id}`

Remove an agent.

#### `POST /training/start`

Launch a training job.

```json
// Request
{
  "agent_id": "ppo_v5",
  "total_timesteps": 1000000,
  "opponent_agent_id": "greedy_v1",
  "deck_pool": ["ST1", "BT14"],
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
        "mean_episode_length": 87
      },
      "started_at": "2026-02-06T..."
    }
  ]
}
```

#### `POST /training/jobs/{job_id}/stop`

Stop a running training job and save the current checkpoint.

#### `GET /training/jobs/{job_id}/metrics`

Get training metrics (reward curve, loss, episode stats) for charting.

```json
{
  "metrics": [
    {"timestep": 10000, "mean_reward": -0.5, "mean_ep_length": 120},
    {"timestep": 20000, "mean_reward": -0.3, "mean_ep_length": 105},
    // ...
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
  "num_games": 100,
  "deck_pool": ["ST1"]
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

## 5. Frontend Architecture

### 5.1 Directory Structure

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
│   │   ├── replayApi.ts         # Replay CRUD
│   │   ├── agentApi.ts          # Agent management
│   │   └── trainingApi.ts       # Training jobs
│   ├── ws/
│   │   └── gameSocket.ts        # WebSocket connection manager
│   ├── stores/
│   │   ├── gameStore.ts         # Interactive game state (Zustand)
│   │   ├── replayStore.ts       # Replay playback state
│   │   ├── uiStore.ts           # UI state (selected cards, modals)
│   │   └── adminStore.ts        # Agent/training state
│   ├── pages/
│   │   ├── HomePage.tsx         # Landing: new game / browse replays / admin
│   │   ├── GamePage.tsx         # Interactive game board
│   │   ├── ReplayPage.tsx       # Replay viewer
│   │   ├── ReplayListPage.tsx   # Browse/filter replays
│   │   ├── AdminPage.tsx        # Agent list + training dashboard
│   │   └── MatchupPage.tsx      # Matchup matrix
│   ├── components/
│   │   ├── board/
│   │   │   ├── GameBoard.tsx    # Root board layout (shared by game + replay)
│   │   │   ├── PlayerHalf.tsx   # One player's zones
│   │   │   ├── BattleArea.tsx   # Field permanents
│   │   │   ├── PermanentSlot.tsx# Single permanent with stack
│   │   │   ├── HandZone.tsx     # Player hand
│   │   │   ├── SecurityStack.tsx
│   │   │   ├── DeckPile.tsx
│   │   │   ├── EggDeck.tsx
│   │   │   ├── BreedingArea.tsx
│   │   │   ├── TrashPile.tsx
│   │   │   └── MemoryGauge.tsx
│   │   ├── game/
│   │   │   ├── ActionBar.tsx    # Contextual action buttons
│   │   │   ├── PhaseIndicator.tsx
│   │   │   ├── GameLog.tsx
│   │   │   └── CardDetail.tsx   # Sidebar card preview
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
│   │   ├── useGameSocket.ts    # WebSocket hook
│   │   ├── useActionMask.ts    # Parse mask into actionable UI state
│   │   └── useReplayPlayer.ts  # Playback timer logic
│   ├── utils/
│   │   ├── actionDecoder.ts    # Decode action ID → human description
│   │   └── constants.ts        # Action ranges, phase names
│   └── types/
│       ├── game.ts             # GameState, PlayerState, PermanentInfo
│       ├── replay.ts           # ReplayData, Frame
│       └── admin.ts            # Agent, TrainingJob, Matchup
```

### 5.2 Key Libraries

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

### 5.3 Card Rendering

Cards need images. Options in priority order:

1. **Placeholder cards** — Colored rectangles with card name, level, DP, cost text. Good enough for development.
2. **Local assets** — Download card images and serve from `/public/cards/`.
3. **CDN proxy** — Proxy card images from an external source through the backend.

The `Card` component should accept a `cardId` and render whatever is available, falling back gracefully.

---

## 6. Implementation Phases

### Phase 1: Foundation
- Set up React + Vite + TypeScript project in `frontend/`
- Implement `GET /cards` endpoint
- Build `Card` component with placeholder rendering
- Build `GameBoard` layout with all zones (static/mock data)
- Build `MemoryGauge`, `PhaseIndicator`, `GameLog` components

### Phase 2: Interactive Game
- Implement WebSocket endpoint `/ws/game/{game_id}`
- Build `useGameSocket` hook
- Implement click-to-act interaction flow
- Build `ActionBar` with contextual buttons derived from action mask
- Build `useActionMask` hook (translates 2120 mask → UI-friendly action list)
- Connect `GamePage` end-to-end: create game → play → game over

### Phase 3: Replay System
- Implement replay recording in backend (wrap HeadlessGame)
- Implement `/replays` CRUD endpoints
- Build `PlaybackControls` + `Timeline` components
- Build `ReplayPage` reusing `GameBoard` in read-only mode
- Build `ReplayListPage` with filtering

### Phase 4: Admin Dashboard
- Implement `/agents` CRUD endpoints
- Implement `/training/start`, `/training/jobs` endpoints
- Build agent list and detail views
- Build training job launch form
- Build metrics chart (reward curve over timesteps)
- Implement `/matchup` endpoints
- Build matchup matrix view

### Phase 5: Polish
- Card images (replace placeholders)
- Attack arrows (SVG overlay)
- Suspend/unsuspend animations
- Sound effects
- Mobile-responsive layout
- Error handling and reconnection logic

---

## 7. Backend Changes Summary

### New files to create:

| File | Purpose |
|------|---------|
| `digimon_gym/api_ws.py` | WebSocket game handler |
| `digimon_gym/replay.py` | Replay recording and storage |
| `digimon_gym/agents/registry.py` | Agent registration and loading |
| `digimon_gym/training/manager.py` | Training job lifecycle |
| `digimon_gym/training/metrics.py` | Metrics collection during training |

### Modifications to existing files:

| File | Changes |
|------|---------|
| `digimon_gym/api.py` | Add card, replay, agent, training, matchup endpoints; mount WebSocket |
| `digimon_gym/engine/game.py` | Add `action_description()` method for human-readable action logs |
| `digimon_gym/engine/runners/headless_game.py` | Add replay recording hooks |
| `digimon_gym/engine/data/card_database.py` | Add `to_dict()` for card API serialization |

### Data storage:

For the pre-alpha stage, use **file-based storage** (JSON files):
- `data/replays/` — One JSON file per replay
- `data/agents.json` — Agent registry
- `data/training_jobs.json` — Training job history
- `models/` — Saved model files (`.zip` for SB3)

Migrate to SQLite or PostgreSQL when needed.

---

## 8. Action Mask → UI Mapping

The 2120-element action mask needs to be translated into UI-friendly actions. The `useActionMask` hook does this:

```typescript
interface ParsedActions {
  canPlay: { handIndex: number; cardId: string }[];
  canTrash: { handIndex: number; cardId: string }[];
  canHatch: boolean;
  canMoveFromBreeding: boolean;
  canPass: boolean;
  canAttack: { attackerSlot: number; targets: { slot: number; isPlayer: boolean }[] }[];
  canDigivolve: { handIndex: number; targets: number[] }[];
  canActivateEffect: { sourceSlot: number; effectIndex: number }[];
  canSelectSource: { fieldSlot: number; sourceIndex: number }[];
}

function parseActionMask(mask: number[], handCards: string[]): ParsedActions {
  // Actions 0-29: play from hand
  // Actions 30-59: trash from hand
  // Action 60: hatch
  // Action 61: move from breeding
  // Action 62: pass
  // Actions 100-399: attack (slot * 15 + target)
  // Actions 400-999: digivolve (hand * 15 + field)
  // Actions 1000-1999: effect (source * 10 + effectIdx)
  // Actions 2000-2119: source selection (field * 10 + sourceIdx)
}
```

This parsed structure drives which UI elements are interactive (highlighted, clickable) at any given moment.
