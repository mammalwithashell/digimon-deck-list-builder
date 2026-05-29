# Architecture Reference

Detailed architecture documentation extracted from CLAUDE.md. For project overview, commands, and working rules, see [CLAUDE.md](../CLAUDE.md).

## Key Repository Paths

- `code/engine_py_legacy/engine/game.py`: core rules engine, tensor writer, action mask, action decoder
- `code/engine_py_legacy/engine/data/enums.py`: phase and enum definitions
- `code/digimon_gym/digimon_gym.py`: `DigimonEnv` and compatibility wrapper
- `code/digimon_gym/agents/pilot_training.py`: MLP/LSTM pilot training entrypoint
- `code/digimon_gym/agents/maskable_recurrent/`: custom recurrent+mask PPO stack
- `code/digimon_gym/agents/gauntlet.py`: MetaGauntlet threat-index opponent sampling
- `code/server/workers/gauntlet_orchestrator.py`: 3-stage training pipeline
- `code/digimon_gym/agents/league_wrapper.py`: PFSP opponent wrapper
- `code/digimon_gym/agents/deck_pool.py`: deck variant generation for training
- `code/digimon_gym/agents/training_metrics.py`: file-based training run metadata (no DB)
- `code/server/workers/training_worker.py`: async DB-backed training job queue
- `code/server/api.py`: app assembly and router registration
- `code/server/routers/`: gameplay-facing routers
- `code/server/db/routers/`: auth/decks/friends/users/issues/admin routers
- `code/server/ai/`: dispatcher, worker, retrieval, batch orchestrator, apply engine
- `code/frontend/src/App.tsx`: route map
- `code/frontend/src/pages/`: primary UI pages
- `code/frontend/src/api/`: backend API clients
- `code/engine_py_legacy/engine/state_filter.py`: per-recipient hidden information filtering for network play
- `code/digimon_gym/inference/onnx_policy.py`: ONNX-based inference wrapper (no PyTorch required)
- `code/server/routers/ws_manager.py`: WebSocket connection manager for PvP games
- `code/server/routers/ws_games.py`: WebSocket game endpoint (player/spectator)
- `code/server/routers/lobby.py`: game lobby with join codes and public game browser
- `code/src-tauri/`: Tauri v2 desktop app shell — Rust-only; hosts gameplay, ONNX inference, deck tools, and the runtime-downloaded model cache
- `code/tools/export_onnx.py`: SB3 → ONNX model conversion (MLP + LSTM)
- `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md`, `AGENTS.md`, `docs/TRAINING_RUNBOOK.md`: behavior contracts
- `docs/TOOLS.md`: card registry, autoencoder, tensor layout, and new-set workflow documentation
- `code/engine_py_legacy/engine/data/tensor_layout.py`: card ID / scalar position map for FeaturesExtractor
- `code/engine_py_legacy/engine/data/card_features.py`: card feature vectorizer for autoencoder
- `code/engine_py_legacy/engine/data/card_registry.py`: card ID → integer index mapping
- `code/tools/build_registry.py`: append-only card registry builder (DigimonCard.io API)
- `code/tools/ingest_cards.py`: card metadata ingestion from DigimonCard.io API
- `code/tools/train_card_autoencoder.py`: warm-start embedding generator
- `code/tools/ingest_pinecone.py` / `code/tools/verify_pinecone.py`: Pinecone vector DB management
- `code/tools/meta_loader.py`: meta deck data loader
- `code/tools/check_frozen_integrity.py`: AI-pipeline frozen-script hash guard (run by `code/server/ai/autofix_apply.py` after each script edit; sunset alongside the Python engine)
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

- Action space size: `2192`
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

`code/server/api.py` mounts:

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
- Export script: `code/tools/export_onnx.py` converts SB3 .zip → .onnx (requires PyTorch)

### Admin AI Routes

`/admin/*` currently supports:

- AI tasks (`/ai-tasks` create/list/get/retry/apply-fix)
- AI batches (`/ai-batches` create/preview/list/detail/cancel)
- Issue queueing (`/issues/{issue_id}/queue-fix`)
- Promotions (`/promotions`, task promotion)
- Engine backlog (`/engine-backlog`)

## Frontend Surface

### Routes

`code/frontend/src/App.tsx` defines:

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

Board components (`code/frontend/src/components/board/`):
- `GameBoard`: top-level board composition (opponent hand → opponent field → memory gauge → player field → player hand)
- `HandZone` + `DraggableHandCard`: hand cards with drag-and-drop, stat overlays (cost/level/DP badges), and hover index callbacks
- `MemoryGauge`: DCGO-style diamond gauge with preview cost ghost indicators on card hover
- `BattleArea`: 14-slot grid with card entry/exit animation tracking (`animate-card-play-in`)
- `PlayerHalf`: per-player field layout (egg deck, breeding, battle area, deck/security/trash piles)
- `PermanentSlot`: individual field card with overlay badges (DP, level, keywords, SA modifier)

Game overlay components (`code/frontend/src/components/game/`):
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
- Frontend `HandCardInfo` type in `code/frontend/src/types/game.ts`
- Used for: stat overlays on hand cards, memory cost preview on hover

### Game Animations

CSS keyframes defined in `code/frontend/src/index.css`:
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

- `client.ts` exports `default` (remote hosted API) and `getGameClient()` used by the web-fallback branches in `gameApi.ts` / `deckApi.ts`
- `useWebSocketGame.ts`: WebSocket hook for PvP/spectating with reconnection; exposes `sendAction` and `sendSurrender`
- `gameApi.ts`: game REST client; desktop builds short-circuit to `rustGameApi.ts` (Tauri `invoke()`) before the HTTP client runs
- `deckApi.ts`: deck REST client; desktop builds short-circuit to `rust_parse_deck` / `rust_validate_deck_raw` / `rust_list_tested_cards`
- `lobbyApi.ts`: lobby REST client (hosted API only)
- `desktopModelsApi.ts`: Tauri-only commands for the model manifest/cache (desktop builds only)

### Frontend Action/Phase Constants

- `code/frontend/src/utils/constants.ts`
- `code/frontend/src/utils/actionDecoder.ts`

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

## Desktop Distribution (Tauri v2)

### Architecture

The desktop app is **Python-free**. Gameplay, ONNX inference, and deck
tooling run inside the embedded `digimon-engine` crate; the frontend
reaches them via Tauri `invoke()`. Online features (PvP, auth, lobby)
still proxy to the hosted FastAPI server over HTTPS.

Trained AI models are fetched at runtime: the Models page GETs
`${MANIFEST_URL}/models/manifest.json`, the user downloads one (or
more), and each is cached under `dirs::data_dir()/digimon-tcg/models/
<sanitized_id>/{policy.onnx, meta.json}`. A compatibility gate checks
the manifest entry's `tensor_size` / `action_space_size` against the
engine contract before the download starts so a tensor-shape change in
`digimon-engine` can't silently break previously-cached models.

### Key Files

- `code/src-tauri/tauri.conf.json`: build config (no `externalBin`, no bundled resources)
- `code/src-tauri/src/main.rs`: Rust entry point — just registers managed state and Tauri commands
- `code/src-tauri/src/engine_commands.rs`: gameplay commands, agent loop, ONNX-policy plumbing
- `code/src-tauri/src/inference_state.rs`: ONNX session cache keyed by model_id
- `code/src-tauri/src/models.rs`: manifest fetch + SHA-verified streaming download cache
- `code/src-tauri/src/deck_commands.rs`: `rust_parse_deck`, `rust_validate_deck_raw`, `rust_list_tested_cards`
- `code/digimon-engine/src/inference/`: MLP + LSTM ONNX policies (mirrors `onnx_policy.py` semantics)
- `code/digimon-engine/src/deck_tools.rs`: deck parser + validator + alpha-pool allowlist
- `code/frontend/src/api/rustGameApi.ts`, `deckApi.ts`, `desktopModelsApi.ts`: Tauri-`invoke` adapters
- `code/frontend/src/pages/ModelsPage.tsx`: manifest browser, download progress, per-seat model selection

### Publishing Models (Ops)

Models land on desktop clients through the hosted API's `/admin/models` +
`/models/manifest.json` endpoints, backed by DigitalOcean Spaces (S3-compatible
object storage). Admin publish flow:

1. `POST /admin/models` — admin creates a pending row and receives a presigned
   PUT URL (signature-v4, 15-min expiry, `ACL=public-read`).
2. Admin PUTs the exported `.onnx` directly to the presigned URL.
3. `POST /admin/models/{id}/confirm` — backend streams the object once, records
   SHA256 + size, inspects the ONNX to capture `tensor_size` / `action_space_size`,
   and transitions the row to `uploaded`.
4. `PATCH /admin/models/{id}` with `{"published": true}` — only accepted once
   the row is in `uploaded` state.
5. Desktop clients `GET /models/manifest.json`, filtered to published +
   uploaded rows, each entry carrying an absolute download URL.

Required env vars on the hosted API: `SPACES_ENDPOINT`, `SPACES_BUCKET`,
`SPACES_REGION`, `SPACES_KEY`, `SPACES_SECRET`. Set `SPACES_CDN_URL` to a
Spaces CDN base (e.g.
`https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com`) to serve manifest
downloads through the edge cache; when unset, the manifest falls back to the
path-style origin URL (`{SPACES_ENDPOINT}/{bucket}/{key}`). The desktop client
treats the manifest URL as opaque — switching between origin and CDN requires
no client change because SHA256 verification matches byte-for-byte either way.
See [ENVIRONMENT.md → Model publishing](ENVIRONMENT.md#model-publishing--digitalocean-spaces)
for the full reference and `.env.example` for the seed block.

### Window sizing and canvas scaling

The desktop window is locked to a fixed list of DCGO-matched resolution
presets (1024×576 → 5160×2160) selectable from the **Graphics Settings**
page (`code/frontend/src/pages/GraphicsSettingsPage.tsx`,
`/settings/graphics` route, desktop-only). User edge-resize is disabled
(`resizable: false` in `tauri.conf.json`); presets and the fullscreen
toggle are the only paths to change window size.

The entire desktop game UI is wrapped in `<CanvasScaler>`
(`code/frontend/src/components/desktop/CanvasScaler.tsx`), which renders
all children inside a fixed 1920×1080 internal canvas and applies a
uniform `transform: scale(min(w/1920, h/1080))` to fit whatever preset
the user picked. Ultrawide presets (3440×1440) letterbox with side bars
rather than stretching. **The board's CSS contains no media queries** —
all layout authoring targets the 1920×1080 canvas; smaller windows just
shrink it uniformly.

Persistence lives in two places:

- `localStorage` (`desktop.graphicsPreset`, `desktop.fullscreen`) is the
  source of truth for the user's selected preset, hydrated by
  `useUiStore` on mount and applied via `@tauri-apps/api/window`'s
  `appWindow.setSize` / `setFullscreen`.
- `tauri-plugin-window-state` restores window *position* across
  launches so multi-monitor users find the window where they left it.

### Working Rules for Desktop

1. The Tauri build must not link any Python runtime. All gameplay, inference, and deck tooling dispatch through Tauri `invoke()` into `digimon-engine`.
2. Frontend desktop gating uses a single flag: `IS_DESKTOP = VITE_BUILD_TARGET === 'desktop'`. Every desktop-only API path and UI surface checks this.
3. Model cache lives under `dirs::data_dir()/digimon-tcg/models/<id>/`; downloads verify SHA256 + content-length before atomic rename from `.tmp`.
4. The compatibility gate (`models_engine_contract` + `assert_compatible`) must run before any download so tensor/action-space drift from engine changes fails loudly, not silently.
5. LSTM ONNX policies must call `reset()` at episode boundaries — same rule as SB3/Python.
6. Game-board CSS targets a fixed 1920×1080 internal canvas. **Do not add `@media` queries inside the board** — `<CanvasScaler>` produces the only viewport sizing the board needs to see. Resolution variation is a uniform `transform: scale()`, not a layout reflow.
7. Window-size changes go through `useUiStore.setGraphicsPreset()` + `appWindow.setSize()`. Do not call `setSize` from random call sites; the store is the source of truth.
