# Architecture Reference

Detailed architecture documentation extracted from CLAUDE.md. For project overview, commands, and working rules, see [CLAUDE.md](../CLAUDE.md).

## Key Repository Paths

- `digimon_gym/engine/game.py`: core rules engine, tensor writer, action mask, action decoder
- `digimon_gym/engine/data/enums.py`: phase and enum definitions
- `digimon_gym/digimon_gym.py`: `DigimonEnv` and compatibility wrapper
- `digimon_gym/agents/pilot_training.py`: MLP/LSTM pilot training entrypoint
- `digimon_gym/agents/maskable_recurrent/`: custom recurrent+mask PPO stack
- `digimon_gym/agents/gauntlet.py`: MetaGauntlet threat-index opponent sampling
- `digimon_gym/agents/gauntlet_orchestrator.py`: 3-stage training pipeline
- `digimon_gym/agents/league_wrapper.py`: PFSP opponent wrapper
- `digimon_gym/agents/deck_pool.py`: deck variant generation for training
- `digimon_gym/agents/training_metrics.py`: file-based training run metadata (no DB)
- `digimon_gym/agents/training_worker.py`: async DB-backed training job queue
- `digimon_gym/api.py`: app assembly and router registration
- `digimon_gym/routers/`: gameplay-facing routers
- `digimon_gym/db/routers/`: auth/decks/friends/users/issues/admin routers
- `digimon_gym/ai/`: dispatcher, worker, retrieval, batch orchestrator, apply engine
- `frontend/src/App.tsx`: route map
- `frontend/src/pages/`: primary UI pages
- `frontend/src/api/`: backend API clients
- `digimon_gym/engine/state_filter.py`: per-recipient hidden information filtering for network play
- `digimon_gym/engine/onnx_policy.py`: ONNX-based inference wrapper (no PyTorch required)
- `digimon_gym/routers/ws_manager.py`: WebSocket connection manager for PvP games
- `digimon_gym/routers/ws_games.py`: WebSocket game endpoint (player/spectator)
- `digimon_gym/routers/lobby.py`: game lobby with join codes and public game browser
- `digimon_gym/desktop_main.py`: lightweight desktop sidecar entry point (no DB/auth)
- `src-tauri/`: Tauri v2 desktop app shell (Rust sidecar lifecycle management)
- `tools/export_onnx.py`: SB3 → ONNX model conversion (MLP + LSTM)
- `tools/build-sidecar.sh`: desktop sidecar build pipeline (PyInstaller + Tauri naming)
- `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md`, `AGENTS.md`, `docs/TRAINING_RUNBOOK.md`: behavior contracts
- `docs/plans/DESKTOP_DISTRIBUTION_PLAN.md`: full implementation plan for desktop distribution
- `docs/TOOLS.md`: card registry, autoencoder, tensor layout, and new-set workflow documentation
- `digimon_gym/engine/data/tensor_layout.py`: card ID / scalar position map for FeaturesExtractor
- `digimon_gym/engine/data/card_features.py`: card feature vectorizer for autoencoder
- `digimon_gym/engine/data/card_registry.py`: card ID → integer index mapping
- `tools/build_registry.py`: append-only card registry builder (DigimonCard.io API)
- `tools/ingest_cards.py`: card metadata ingestion from DigimonCard.io API
- `tools/train_card_autoencoder.py`: warm-start embedding generator
- `tools/transpile_dcgo.py`: C#→Python card effect transpiler
- `tools/transpiler/`: transpiler package
- `tools/ingest_pinecone.py` / `tools/verify_pinecone.py`: Pinecone vector DB management
- `tools/meta_loader.py`: meta deck data loader
- `tools/check_frozen_integrity.py`: CI frozen script integrity guard
- `qa/archetype-qa/`: archetype QA reports, engine API reference, engine gaps
- `qa/qa-reports/`: gameplay QA test reports, validated cards index
- `DCGO/`: git submodule — DCGO C# source (reference implementation)

## RL and Game Contracts

### Environment API

`DigimonEnv` (Gymnasium):

- `reset(seed=None, options=None) -> (obs, info)`
- `step(action) -> (obs, reward, terminated, truncated, info)`
- `action_mask() -> np.ndarray[int8]`
- `info['action_mask']` is returned from reset/step

### Reward Shaping

- Terminal: win `+1.0`, loss `-1.0`, draw `0.0`
- Dense (per-step): security delta `* 0.01`, board DP delta `* 0.0001`
- Bounty bonus (via GauntletWrapper): configurable on terminal wins vs high-TI opponents

### Tensor Contract

- Tensor size: `1375` (compact layout with integer card IDs)
- Card identities are integer registry indices (1 float per card)
- `nn.Embedding` lookup happens inside `CardEmbeddingExtractor` on the GPU
- `FIELD_SLOTS=14`, `MAX_SOURCES=11`, `SLOT_SIZE=40`
- See `docs/TENSOR_SPEC.md` for exact layout

### Action Contract

- Action space size: `2168`
- `SECURITY_TARGET=14`, `BREEDING_SLOT=14` (= `FIELD_SLOTS`)
- `SOURCES_PER_FIELD=12` (stride for source selection)
- Phase-aware decoding in `Game.decode_action`
- See `docs/ACTION_SPEC.md` for ranges and conventions

### Phase Coverage

Current `GamePhase` values include core, selection, and interrupt phases:

- `Start`, `Draw`, `Breeding`, `Main`, `End`
- `SelectTarget`, `SelectMaterial`, `SelectTrash`, `SelectSource`, `SelectHand`, `SelectReveal`, `SelectEffectChoice`, `SelectSecurity`
- `BlockTiming`, `CounterTiming`
- `EndOfTurnAction`, `AllianceTiming`
- `Mulligan` (value 17)

### Wrapper Chain

Training wrapper chain (innermost to outermost):

```
DigimonEnv → OpponentWrapper → DeckPoolWrapper → GauntletWrapper → ActionMasker
```

- `OpponentWrapper`: converts 2-player game to single-agent MDP
- `DeckPoolWrapper`: varies agent's own deck per episode
- `GauntletWrapper`: samples opponent decks from MetaGauntlet
- `ActionMasker`: SB3 mask interface

Full details in `AGENTS.md` §2.4.

### Training Pipeline

- MetaGauntlet: threat-index weighted opponent sampling (see `AGENTS.md` §3)
- GauntletOrchestrator: 3-stage pipeline — bootstrap, meta-weighted/PFSP, round-robin evaluation
- Training operations: see `docs/TRAINING_RUNBOOK.md`

## Backend API Surface

### App Assembly

`digimon_gym/api.py` mounts:

- DB-backed routers:
  - `/auth/*`
  - `/users/*`
  - `/decks/*`
  - `/friends/*`
  - `/assets/*`
  - `/issues/*`
  - `/admin/*`
- Domain routers:
  - `/health`
  - `/simulations`
  - `/games`, `/games/models`
  - `/recordings`
  - `/replays`
  - `/decks/parse`, `/decks/validate` (deck tools)
  - `/lobby/*` (create/join/list/cancel)
  - `/ws/games/{id}` (WebSocket PvP + spectating)

### Gameplay Routes

Primary routes include:

- Game session lifecycle: `/games`, `/games/{id}/actions`, `/games/{id}/steps`, `/games/{id}/state`, `/games/{id}/action-mask`, `/games/{id}/actions`, `/games/{id}/logs`, `/games/{id}`, `/games/{id}/surrender`
- Recording/replay:
  - `/games/{id}/recording`, `/games/{id}/recordings`
  - `/recordings/*`
  - `/replays/*`

Legacy aliases are present in several routers for compatibility.

### WebSocket PvP & Spectating

- `/ws/games/{id}?token=JWT&role=player|spectator` — real-time game transport
- `ConnectionManager` (ws_manager.py) tracks players/spectators per game
- `state_filter.py` provides per-recipient hidden information filtering:
  - Players see own hand, opponent's hand hidden (count only), both security stacks hidden
  - Spectators in `"hidden"` mode see redacted state; `"open"` mode shows everything
- Message protocol: `state_update`, `player_joined`, `player_disconnected`, `game_over`, `error`, `surrender`
- Surrender: client sends `{type: "surrender"}`, server broadcasts `game_over` with `surrendered_by` field
- Reconnection: frontend hook retries with exponential backoff (1s–30s, max 5 retries)

### Lobby System

- `POST /lobby/create` — creates pending game with 6-char join code (requires auth)
- `POST /lobby/join/{code}` — joins and starts InteractiveGame (both humans)
- `GET /lobby/games` — lists public pending games
- `DELETE /lobby/{id}` — host cancels pending game
- In-memory storage (`pending_games` dict)

### ONNX Agent Inference

- `CreateGameRequest` supports `player1_policy="trained"` with `player1_model="model.onnx"`
- ONNX models resolved from `ONNX_MODELS_DIR` env var (default: `models/`)
- `GET /games/models` — lists available `.onnx` files
- Model type auto-detected from filename: `*lstm*` → `OnnxLstmPolicy`, else `OnnxMlpPolicy`
- Path traversal protection via `Path.name` sanitization
- Export script: `tools/export_onnx.py` converts SB3 .zip → .onnx (requires PyTorch)

### Admin AI Routes

`/admin/*` currently supports:

- AI tasks (`/ai-tasks` create/list/get/retry/apply-fix)
- AI batches (`/ai-batches` create/preview/list/detail/cancel)
- Issue queueing (`/issues/{issue_id}/queue-fix`)
- Promotions (`/promotions`, task promotion)
- Engine backlog (`/engine-backlog`)

## Frontend Surface

### Routes

`frontend/src/App.tsx` defines:

- Public: `/`, `/login`, `/register`
- Auth-guarded: `/game/:id?`, `/deckbuilder/:id?`, `/lobby`
- Admin role-guarded: `/admin/issues`, `/admin/tasks`, `/admin/promotions`, `/admin/barracks`, `/admin/arena`, `/admin/gauntlet/:id?`, `/admin/deck-pools/:id?`

### Main Pages

- `GamePage`: play/session UI (dual-mode: HTTP for local games, WebSocket for PvP/spectating)
- `LobbyPage`: multiplayer lobby (create/join/browse tabs)
- `DeckBuilderPage`: deck editing and validation
- `AdminIssuesPage`, `AdminTasksPage`, `AdminPromotionsPage`: admin AI workflow UI
- `BarracksPage`, `ArenaPage`, `GauntletPage`, `DeckPoolPage`: training management UI

### Game UI Components

Board components (`frontend/src/components/board/`):
- `GameBoard`: top-level board composition (opponent hand → opponent field → memory gauge → player field → player hand)
- `HandZone` + `DraggableHandCard`: hand cards with drag-and-drop, stat overlays (cost/level/DP badges), and hover index callbacks
- `MemoryGauge`: DCGO-style diamond gauge with preview cost ghost indicators on card hover
- `BattleArea`: 14-slot grid with card entry/exit animation tracking (`animate-card-play-in`)
- `PlayerHalf`: per-player field layout (egg deck, breeding, battle area, deck/security/trash piles)
- `PermanentSlot`: individual field card with overlay badges (DP, level, keywords, SA modifier)

Game overlay components (`frontend/src/components/game/`):
- `ActionBar`: phase-aware action buttons + surrender button (with confirmation dialog)
- `ResultOverlay`: win/loss/draw/surrender result screen
- `PhaseBanner`: full-screen phase transition banner (1.2s `bannerSlide`)
- `DigivolveBanner`: digivolution cut-in overlay (1.4s) with color-matched glow and card drop animation
- `BattleEffect`: CSS slash overlay + shake on losing permanent's slot after battle resolution
- `CardOverlay`: DCGO-style vertical stack inspector for viewing permanent sources
- `SecurityRevealOverlay`: security card reveal with flip animation
- `EffectPopup`: floating effect activation indicator
- `AttackArrow`: SVG arrow drawn between attacker and target slots
- `SelectionPanel`, `PromptBar`, `KeywordPromptDialog`: selection phase UI
- `TrashViewer`: modal trash pile browser

### Hand Card Data Flow

Backend `player_ui_data()` sends both `handIds` (string[]) and `handCards` (metadata array):
```
handCards[]: { cardId, cardName, playCost, level, dp, colors[], cardKind, evoCosts[] }
```
- `state_filter.py` redacts both `handIds` and `handCards` for opponents (count preserved)
- Frontend `HandCardInfo` type in `frontend/src/types/game.ts`
- Used for: stat overlays on hand cards, memory cost preview on hover

### Game Animations

CSS keyframes defined in `frontend/src/index.css`:
- `cardPlayIn` (0.35s): scale bounce + Y translation for cards entering field
- `cardTrashOut` (0.3s): shrink + fade for cards leaving field
- `digivolveBanner` (1.4s): horizontal scale-in/out for digivolve cut-in
- `digivolveCardDrop` (0.5s): card falling into digivolve banner
- `battleSlash` (0.35s): diagonal clip-path wipe over losing slot
- `battleShake` (0.4s): rapid position jitter on losing permanent
- `securityBreak` (0.6s): pulse + red border for security checks
- `bannerSlide` (1.2s): phase banner entrance/exit
- `effectPulse` (1.2s): golden glow ring for active effects

### Surrender

- Backend: `Game.surrender(player_id)` emits `surrender` event then calls `declare_winner()`
- HTTP: `POST /games/{id}/surrender` with `{player_id: 1|2}`
- WebSocket: client sends `{type: "surrender"}`, server broadcasts `game_over` with `surrendered_by`
- Frontend: red "Surrender" button in `ActionBar` (far right), `window.confirm()` guard
- `ResultOverlay` shows "Surrendered" / "Opponent surrendered" based on `surrenderedBy` state

### Frontend API Architecture

- `client.ts` exports `default` (remote server) and `getGameClient()` for Tauri dual-server routing
- `useWebSocketGame.ts`: WebSocket hook for PvP/spectating with reconnection; exposes `sendAction` and `sendSurrender`
- `gameApi.ts`: game REST client (create, action, step, state, mask, log, surrender, delete)
- `lobbyApi.ts`: lobby REST client
- In Tauri desktop mode, local game requests route to sidecar (`localhost:8321`); online features to remote server

### Frontend Action/Phase Constants

- `frontend/src/utils/constants.ts`
- `frontend/src/utils/actionDecoder.ts`

Keep these aligned with backend constants.

## Admin AI Workflow (Current)

Core modules:

- `dispatcher.py`: task-specific prompt+schema dispatch
- `worker.py`: DB-backed queue loop and execution
- `batch_orchestrator.py`: batch creation/scheduling/guards/finalization
- `autofix_apply.py`: scoped edit validation + apply + checks
- `git_adapter.py`: worktree/branch/commit/PR automation

Common task types:

- `review_batch`
- `qa_analysis`
- `engine_audit`
- `script_autofix`

Common scope profiles:

- `script`
- `script_engine`
- `script_engine_transpiler`

## Desktop Distribution (Tauri v2)

### Architecture

The desktop app uses a **dual-server** model:
- **Local sidecar** (PyInstaller binary): game engine + deck tools only, no DB/auth. For offline play against agents.
- **Remote server**: PvP, auth, lobby, user data. All online features.

### Key Files

- `src-tauri/tauri.conf.json`: build config, sidecar + resource bundling
- `src-tauri/src/main.rs`: Rust entry point, spawns/kills Python sidecar
- `digimon_gym/desktop_main.py`: stripped-down FastAPI app (no SQLAlchemy imports)
- `desktop.spec`: PyInstaller spec excluding heavy deps (torch, SB3, SQLAlchemy)
- `tools/build-sidecar.sh`: build script with `gameplay` (no models) and `full` (ONNX) profiles
- `requirements-desktop.txt`: minimal deps for sidecar

### Build Profiles

- `gameplay` (default): greedy/random bots only, ~60-90MB
- `full`: auto-exports SB3 → ONNX, bundles models, ~90-120MB

### Working Rules for Desktop

1. `desktop_main.py` must never import from `digimon_gym.db.*` or `digimon_gym.ai.*`
2. The sidecar has its own inline game routes (not shared with `games.py`) to avoid DB import chains
3. ONNX model paths are resolved at game creation time via `ONNX_MODELS_DIR` env var
4. The Rust sidecar manager pipes stdout/stderr for debugging; sidecar is killed on window close
