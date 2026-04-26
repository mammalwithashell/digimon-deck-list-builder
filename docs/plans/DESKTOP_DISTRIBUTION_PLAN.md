# Implementation Plan: Network PvP, ONNX Inference, and Tauri Desktop

Three separate PRs, each building on the previous. Game codes first for matchmaking, lobby browsing added afterward.

---

## PR 1: WebSocket PvP + Spectating

### Overview

Add a WebSocket transport layer on top of the existing game engine for network player-vs-player games and live spectating. Existing HTTP-based local play (human vs agent) continues to work unchanged.

### 1.1 Backend: Connection Manager

**New file: `code/server/routers/ws_manager.py`**

```python
class ConnectionManager:
    """Tracks WebSocket connections per game."""

    # game_id → {player_id → WebSocket}
    players: dict[str, dict[int, WebSocket]]
    # game_id → [WebSocket, ...]
    spectators: dict[str, list[WebSocket]]

    async def connect_player(game_id, player_id, ws) -> None
    async def connect_spectator(game_id, ws) -> None
    async def disconnect(game_id, ws) -> None
    async def broadcast_state(game_id, runner, game_settings) -> None
    async def send_to_player(game_id, player_id, payload) -> None
```

Responsibilities:
- Track which WebSocket belongs to which player in which game
- **Send perspective-filtered state to each recipient** (see §1.9 Hidden Information)
- Handle disconnection cleanup
- Support reconnection (player reconnects with same token/player_id)

### 1.2 Backend: WebSocket Game Router

**New file: `code/server/routers/ws_games.py`**

```python
router = APIRouter()

@router.websocket("/ws/games/{game_id}")
async def game_websocket(ws: WebSocket, game_id: str, token: str, role: str):
    """
    WebSocket endpoint for live game participation or spectating.

    Query params:
      - token: JWT auth token
      - role: "player" or "spectator"
    """
```

**WebSocket message protocol (JSON):**

Client → Server:
```json
{"type": "action", "action_id": 123}
{"type": "ping"}
```

Server → Client:
```json
{"type": "state_update", "state": {...}, "action_mask": [...], "action_descriptions": {...}, "current_player_id": 1, "logs": [...], "is_game_over": false}
{"type": "player_joined", "player_id": 2, "display_name": "..."}
{"type": "player_disconnected", "player_id": 1}
{"type": "player_reconnected", "player_id": 1}
{"type": "spectator_count", "count": 5}
{"type": "error", "message": "Not your turn"}
{"type": "game_over", "winner_id": 1, "state": {...}}
{"type": "pong"}
```

**Action handling logic:**
1. Receive `{"type": "action", "action_id": N}` from a player
2. Validate: is it this player's turn? (`game.current_player_id == sender_player_id`)
3. Validate: is the action legal? (`action_mask[N] == 1`)
4. Execute: `runner.step(action_id)`
5. **Send per-recipient filtered state** via `ConnectionManager.broadcast_state()`:
   - Player 1 gets their own hand/security revealed, opponent's hidden
   - Player 2 gets the inverse
   - Spectators get hands/security hidden for both unless the game allows open spectating

**Spectator handling:**
- Spectators receive **redacted** `state_update` messages (see §1.9 Hidden Information)
- Spectators cannot send `action` messages (server ignores/errors)
- Spectators do NOT receive `action_mask` (they can't act; showing valid moves leaks strategy)

### 1.3 Backend: Game Lobby & Matchmaking

**New file: `code/server/routers/lobby.py`**

```python
router = APIRouter(tags=["lobby"])

# In-memory lobby state
pending_games: dict[str, PendingGame] = {}

class PendingGame(BaseModel):
    game_id: str
    join_code: str          # 6-char alphanumeric
    host_user_id: str
    host_display_name: str
    host_deck: list[str]
    created_at: datetime
    is_public: bool         # visible in lobby listing

@router.post("/lobby/create")
async def create_lobby_game(request: CreateLobbyRequest) -> dict:
    """Host creates a game and gets a join code. Game is not started yet."""
    # Returns: {"game_id": "...", "join_code": "ABC123"}

@router.post("/lobby/join/{join_code}")
async def join_lobby_game(join_code: str, request: JoinLobbyRequest) -> dict:
    """Second player joins via code. Creates the InteractiveGame, returns game_id."""
    # Validates join_code exists, creates InteractiveGame with both decks
    # Returns: {"game_id": "...", "player_id": 2}

@router.get("/lobby/games")
async def list_lobby_games() -> list[dict]:
    """List public pending games (for lobby browser)."""
    # Returns list of {game_id, host_display_name, created_at}

@router.delete("/lobby/{game_id}")
async def cancel_lobby_game(game_id: str) -> dict:
    """Host cancels a pending game."""
```

**Flow:**
1. Player A: `POST /lobby/create` with their deck → gets `join_code: "ABC123"`
2. Player A: connects to `wss://server/ws/games/{game_id}?role=player&token=...` → waits
3. Player B: `POST /lobby/join/ABC123` with their deck → gets `game_id`
4. Player B: connects to same WebSocket → server sends `player_joined` to Player A
5. Server sends initial `state_update` to both → game begins
6. Spectators: connect to same WebSocket with `role=spectator` at any time

### 1.4 Backend: Register New Routers

**Modify: `code/server/api.py`**

- Import and mount `ws_games.router` and `lobby.router`
- Add `websockets` to requirements if not already present (FastAPI includes Starlette WebSocket support natively — no new dependency needed)

### 1.5 Frontend: WebSocket Hook

**New file: `code/frontend/src/hooks/useWebSocketGame.ts`**

```typescript
interface UseWebSocketGameOptions {
  gameId: string;
  token: string;
  role: 'player' | 'spectator';
  onStateUpdate: (payload: StateUpdatePayload) => void;
  onPlayerJoined: (payload: PlayerJoinedPayload) => void;
  onGameOver: (payload: GameOverPayload) => void;
  onError: (message: string) => void;
}

function useWebSocketGame(options: UseWebSocketGameOptions) {
  // Returns: { sendAction, connectionStatus, disconnect }
  // Manages WebSocket lifecycle, reconnection with backoff
  // Calls onStateUpdate which updates the Zustand game store
}
```

**Reconnection strategy:**
- On disconnect: retry with exponential backoff (1s, 2s, 4s, 8s, max 30s)
- On reconnect: server sends current game state as initial `state_update`
- After 5 failures: show "Connection lost" UI, let user manually retry

### 1.6 Frontend: Lobby UI

**New file: `code/frontend/src/pages/LobbyPage.tsx`**

- "Create Game" form: select deck, choose public/private → gets join code
- "Join Game" input: enter join code → joins game
- "Browse Games" list: shows public pending games (polls `GET /lobby/games`)
- After joining: redirect to `/game/{id}` with WebSocket mode

**Modify: `code/frontend/src/App.tsx`**

- Add route: `/lobby` → `LobbyPage`

### 1.7 Frontend: GamePage Dual-Mode Support

**Modify: `code/frontend/src/pages/GamePage.tsx`**

- Detect mode: if game was created via lobby (URL param or route state), use `useWebSocketGame`
- If game was created locally (existing flow), continue using `useGameActions` with HTTP
- The Zustand game store is updated the same way regardless of transport — only the source of updates differs

### 1.8 Frontend: Spectator Mode

**Modify: `code/frontend/src/pages/GamePage.tsx`**

- If `role=spectator` in URL params, connect via WebSocket with `role: "spectator"`
- Hide action buttons (no action mask interaction)
- Render redacted state: show card backs for hidden hands/security, show public zones (battle area, trash, memory, phase)
- Display spectator count

### 1.9 Backend: Hidden Information & Perspective Filtering

**New file: `code/engine_py_legacy/engine/state_filter.py`**

The game engine's `to_ui_json()` currently returns **everything** — both players' hands, security stacks, etc. For network play this leaks hidden information. We need per-recipient state filtering.

```python
def filter_state_for_player(full_state: dict, player_id: int) -> dict:
    """
    Return a copy of the game state filtered for a specific player.
    - Player sees their own hand, security (face-down count only), full board
    - Opponent's hand: card count only (not card identities)
    - Opponent's security: count only
    - Both players' battle areas, trash, breeding: fully visible (public zones)
    - Revealed cards: visible to all (effects that reveal)
    """

def filter_state_for_spectator(full_state: dict, spectator_mode: str) -> dict:
    """
    Return a copy of the game state redacted for spectators.

    spectator_mode controls visibility:
    - "hidden" (default): Both hands and security hidden (card counts only).
        Spectators see board, trash, memory, phase, and publicly revealed cards.
    - "open": Full visibility (opt-in by game host, e.g. for tournament streams
        with delay). Both players must consent at game creation time.
    """
```

**How this integrates with `ConnectionManager.broadcast_state()`:**

```python
async def broadcast_state(self, game_id, runner, game_settings):
    full_state = runner.game.to_ui_json()
    mask = runner.get_action_mask().tolist()
    descriptions = runner.game.describe_actions(runner.game.current_player_id)

    # Each player gets their own filtered perspective
    for player_id, ws in self.players[game_id].items():
        filtered = filter_state_for_player(full_state, player_id)
        player_mask = mask if runner.game.current_player_id == player_id else []
        await ws.send_json({
            "type": "state_update",
            "state": filtered,
            "action_mask": player_mask,
            "action_descriptions": descriptions if player_mask else {},
            "current_player_id": runner.game.current_player_id,
            "is_game_over": runner.is_game_over,
        })

    # Spectators get redacted state, no mask
    spectator_state = filter_state_for_spectator(full_state, game_settings.spectator_mode)
    for ws in self.spectators[game_id]:
        await ws.send_json({
            "type": "state_update",
            "state": spectator_state,
            "current_player_id": runner.game.current_player_id,
            "is_game_over": runner.is_game_over,
        })
```

**Lobby integration — spectator mode setting:**

Add to `CreateLobbyRequest`:
```python
allow_spectators: bool = True
spectator_mode: str = "hidden"  # "hidden" or "open"
```

Both players must agree to `"open"` spectating. The host sets it at game creation; it cannot change mid-game.

**Note on existing HTTP local play:** The current `to_ui_json()` endpoint returns unfiltered state and continues to do so — in local play (human vs agent on the same machine), there's no opponent to hide information from. The filtering only applies to the WebSocket PvP path.

### 1.10 Backend: Reconnection Support

**Modify: `code/server/routers/ws_games.py`**

When a player reconnects:
1. Authenticate via token → identify `user_id` and `player_id`
2. Check if `game_id` exists in `active_games` and player was previously connected
3. Re-register WebSocket in `ConnectionManager`
4. Send current game state as `state_update` message
5. Notify other player/spectators via `player_reconnected` message

### 1.11 Files Summary

| File | Action | Description |
|------|--------|-------------|
| `code/server/routers/ws_manager.py` | Create | WebSocket connection manager |
| `code/server/routers/ws_games.py` | Create | WebSocket game endpoint |
| `code/server/routers/lobby.py` | Create | Lobby create/join/list endpoints |
| `code/engine_py_legacy/engine/state_filter.py` | Create | Per-recipient state filtering (hidden information) |
| `code/server/routers/schemas.py` | Modify | Add lobby request/response schemas |
| `code/server/api.py` | Modify | Mount new routers |
| `code/frontend/src/hooks/useWebSocketGame.ts` | Create | WebSocket hook |
| `code/frontend/src/pages/LobbyPage.tsx` | Create | Lobby UI page |
| `code/frontend/src/pages/GamePage.tsx` | Modify | Dual-mode (HTTP/WS), spectator support, redacted rendering |
| `code/frontend/src/App.tsx` | Modify | Add `/lobby` route |
| `code/frontend/src/api/lobbyApi.ts` | Create | Lobby API client |

### 1.12 Verification

- Unit test: WebSocket connection/disconnection/reconnection with mock game
- Integration test: Two clients connect, exchange actions, verify state consistency
- **Test: Player 1 cannot see Player 2's hand or security contents via WebSocket state**
- **Test: Spectator receives redacted state (card counts only for hands/security)**
- **Test: Open spectator mode (`spectator_mode="open"`) reveals full state to spectators**
- Test: Spectator connects mid-game, receives current redacted state
- Test: Player disconnects and reconnects, game resumes
- Test: Invalid action rejected (wrong turn, masked action)
- Test: Existing HTTP game flow still works (regression)

---

## PR 2: ONNX Inference for Trained Agents

### Overview

Export SB3 models to ONNX format and create a lightweight inference wrapper using `onnxruntime`. This enables playing against trained agents without bundling PyTorch (~200MB savings). Also adds a new `"trained"` policy type to the game router.

### 2.1 Conversion Script

**New file: `scripts/export_onnx.py`**

```python
"""Export SB3 MaskablePPO / MaskableRecurrentPPO models to ONNX format."""

def export_mlp(sb3_zip_path: str, output_path: str) -> None:
    """
    Load MaskablePPO from .zip, trace the policy network, export to ONNX.
    Input: obs (1, 981) float32
    Output: logits (1, 2120) float32
    """

def export_lstm(sb3_zip_path: str, output_path: str) -> None:
    """
    Load MaskableRecurrentPPO, trace policy + LSTM, export to ONNX.
    Inputs: obs (1, 1, 981), h (1, 1, 256), c (1, 1, 256)
    Outputs: logits (1, 2120), h_out (1, 1, 256), c_out (1, 1, 256)
    """
```

This script requires PyTorch (it imports the SB3 model to trace it). It runs on the developer's machine or CI — not on the end user's desktop.

### 2.2 ONNX Inference Wrapper

**New file: `code/digimon_gym/inference/onnx_policy.py`**

```python
"""Lightweight ONNX-based policy for trained agent inference. No PyTorch required."""

import numpy as np
import onnxruntime as ort

class OnnxMlpPolicy:
    """Load an ONNX MLP model and produce actions."""

    def __init__(self, onnx_path: str):
        self.session = ort.InferenceSession(onnx_path)

    def predict(self, obs: np.ndarray, action_mask: np.ndarray) -> int:
        """
        Run inference.
        1. Forward pass: obs → logits
        2. Mask: logits[mask == 0] = -inf
        3. Return: argmax(softmax(masked_logits))
        """

class OnnxLstmPolicy:
    """Load an ONNX LSTM model and produce actions with state threading."""

    def __init__(self, onnx_path: str):
        self.session = ort.InferenceSession(onnx_path)
        self.h = np.zeros((1, 1, 256), dtype=np.float32)
        self.c = np.zeros((1, 1, 256), dtype=np.float32)

    def predict(self, obs: np.ndarray, action_mask: np.ndarray) -> int:
        """
        Run inference with LSTM state.
        1. Forward pass: (obs, h, c) → (logits, h_out, c_out)
        2. Update self.h, self.c
        3. Mask and argmax as above
        """

    def reset(self) -> None:
        """Reset LSTM state at episode boundary."""
        self.h = np.zeros((1, 1, 256), dtype=np.float32)
        self.c = np.zeros((1, 1, 256), dtype=np.float32)
```

### 2.3 New Policy Type in InteractiveGame

**Modify: `code/engine_py_legacy/engine/runners/interactive_game.py`**

Add support for `"trained"` policy alongside `"greedy"` and `"random"`:

```python
class InteractiveGame(BaseGameRunner):
    def __init__(self, ..., player1_policy="greedy", player2_policy="greedy",
                 player1_model_path=None, player2_model_path=None, ...):
        # If policy is "trained", load OnnxMlpPolicy or OnnxLstmPolicy
        # from the provided model_path

    def _select_agent_action(self, ...):
        if policy == "trained":
            obs = self.game.get_board_state_tensor(player_id)
            mask = self.game.get_action_mask(player_id)
            return self.onnx_policy.predict(obs, mask)
        elif policy == "random":
            ...
        else:  # greedy
            ...
```

### 2.4 Game Router Updates

**Modify: `code/server/routers/schemas.py`**

```python
class CreateGameRequest(BaseModel):
    ...
    player1_policy: str = "greedy"  # "greedy", "random", or "trained"
    player2_policy: str = "greedy"
    player1_model: Optional[str] = None  # ONNX model filename
    player2_model: Optional[str] = None
```

**Modify: `code/server/routers/games.py`**

- Pass `model_path` to InteractiveGame when policy is `"trained"`
- Resolve model filename against a configurable models directory
- Validate that the ONNX file exists before creating the game

### 2.5 Dependency Management

**Modify: `requirements.txt`**

Add `onnxruntime>=1.16` (already alongside PyTorch — no conflict)

**New file: `requirements-desktop.txt`**

```
# Desktop sidecar dependencies — game engine only, no DB/auth/ML
numpy>=1.24
fastapi>=0.100
uvicorn>=0.20
pydantic>=2.0
Pillow>=10.0
beautifulsoup4>=4.12
requests>=2.31
python-dotenv>=1.0
onnxruntime>=1.16
```

Note: No SQLAlchemy, aiosqlite, bcrypt, or python-jose — the sidecar doesn't do auth or DB. Those features are on the central server. No websockets either — PvP goes through the remote server directly.

### 2.6 Files Summary

| File | Action | Description |
|------|--------|-------------|
| `scripts/export_onnx.py` | Create | SB3 → ONNX conversion script (requires PyTorch) |
| `code/digimon_gym/inference/onnx_policy.py` | Create | ONNX inference wrapper (no PyTorch) |
| `code/engine_py_legacy/engine/runners/interactive_game.py` | Modify | Add "trained" policy type |
| `code/server/routers/schemas.py` | Modify | Add model fields to CreateGameRequest |
| `code/server/routers/games.py` | Modify | Pass model path to InteractiveGame |
| `requirements.txt` | Modify | Add onnxruntime |
| `requirements-desktop.txt` | Create | Gameplay-only deps |

### 2.7 Verification

- Export a test MLP model to ONNX, run inference, compare output to PyTorch inference (should be identical)
- Export a test LSTM model to ONNX, verify state threading produces same action sequence
- Create a game with `policy="trained"` and play against the ONNX agent
- Verify the backend starts and serves games with only `requirements-desktop.txt` installed (no torch import errors)
- Test that `greedy` and `random` policies still work (regression)

---

## PR 3: Tauri Desktop Shell

### Overview

Package the application as a Tauri v2 desktop app. A **lightweight** Python sidecar (game engine + deck tools only, no DB/auth) is bundled via PyInstaller for offline play against agents. All online features (PvP, auth, decks, friends, lobby) go through the central server. Two build profiles: gameplay-only (greedy bot only, ~60-90MB) and full (with ONNX agents, ~90-120MB).

### 3.1 Initialize Tauri

**New directory: `code/src-tauri/`**

Run `npm create tauri-app` from the frontend directory (or `cargo tauri init`). This generates:
- `code/src-tauri/tauri.conf.json` — build config
- `code/src-tauri/src/main.rs` — Rust entry point
- `code/src-tauri/Cargo.toml` — Rust dependencies
- `code/src-tauri/capabilities/` — permission config
- `code/src-tauri/icons/` — app icons

### 3.2 Tauri Configuration

**New file: `code/src-tauri/tauri.conf.json`**

```jsonc
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-utils/schema.json",
  "productName": "Digimon TCG",
  "version": "0.1.0",
  "identifier": "com.digimon-tcg.app",
  "build": {
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../code/frontend/dist",
    "devUrl": "http://localhost:5173"
  },
  "app": {
    "windows": [
      {
        "title": "Digimon TCG",
        "width": 1280,
        "height": 800,
        "minWidth": 1024,
        "minHeight": 768
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "externalBin": ["binaries/digimon-server"],
    "resources": ["resources/**/*"]
  }
}
```

### 3.3 Tauri Capabilities (Permissions)

**New file: `code/src-tauri/capabilities/default.json`**

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-execute",
    "shell:allow-spawn",
    "fs:allow-read",
    "http:default"
  ]
}
```

### 3.4 Rust Entry Point: Sidecar Management

**New file: `code/src-tauri/src/main.rs`**

```rust
// Minimal Rust — just manages the Python sidecar lifecycle

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Resolve resource paths for ONNX models
            let models_dir = app.path().resolve("resources/models", BaseDirectory::Resource)?;

            // Spawn Python sidecar — lightweight, no DB
            let sidecar = app.shell()
                .sidecar("digimon-server")
                .args(["--port", "8321", "--models-dir", &models_dir])
                .spawn()?;

            // Store handle for cleanup
            app.manage(sidecar);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                // Kill sidecar on app close
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3.5 Python Sidecar Entry Point

**New file: `code/digimon_gym/desktop_main.py`**

The desktop sidecar is a **stripped-down FastAPI app** — it only mounts the game engine routes and deck tools. No database, no auth, no user management. All of that lives on the central server.

```python
"""Desktop sidecar entry point. Lightweight — game engine + deck tools only."""
import argparse
import os
import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

def create_desktop_app(models_dir: str) -> FastAPI:
    app = FastAPI(title="Digimon TCG Desktop")
    app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])

    os.environ["MODELS_DIR"] = models_dir

    # Only mount game engine routes — no DB, no auth, no admin
    from digimon_gym.routers import games, deck_tools
    app.include_router(games.router)
    app.include_router(deck_tools.router)

    @app.get("/health")
    def health():
        return {"status": "ok"}

    return app

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=8321)
    parser.add_argument("--models-dir", default="./models")
    args = parser.parse_args()

    app = create_desktop_app(args.models_dir)
    uvicorn.run(app, host="127.0.0.1", port=args.port)

if __name__ == "__main__":
    main()
```

### 3.6 PyInstaller Spec

**New file: `desktop.spec`**

PyInstaller spec file that bundles:
- `code/engine_py_legacy/engine/` package (game engine only)
- `code/server/routers/games.py`, `deck_tools.py`, `schemas.py`, `state.py`
- `code/digimon_gym/desktop_main.py` (entry point)
- Card data files from `code/engine_py_legacy/engine/data/`
- Excludes: torch, stable-baselines3, sb3-contrib, openai, anthropic, SQLAlchemy, aiosqlite, all training/admin/AI/DB modules

Output binary name follows Tauri's platform-specific naming convention:
- `digimon-server-x86_64-unknown-linux-gnu`
- `digimon-server-aarch64-apple-darwin`
- `digimon-server-x86_64-pc-windows-msvc.exe`

### 3.7 Build Scripts

**New file: `scripts/build-sidecar.sh`**

```bash
#!/bin/bash
# Usage: ./scripts/build-sidecar.sh [gameplay|full]
# Builds PyInstaller binary for the current platform

PROFILE=${1:-gameplay}

if [ "$PROFILE" = "gameplay" ]; then
    pip install -r requirements-desktop.txt
else
    pip install -r requirements.txt
    # Copy model weights to resources
    mkdir -p code/src-tauri/resources/models
    cp models/*.onnx code/code/src-tauri/resources/models/
fi

pyinstaller desktop.spec
# Move output to code/src-tauri/binaries/ with platform-specific name
```

### 3.8 Frontend: Dual-Server Architecture

**Modify: `code/frontend/src/api/client.ts`**

The desktop app talks to **two servers** depending on the feature:

```typescript
// Local sidecar — offline game engine (vs agent, deck tools)
const localBaseURL = window.__TAURI__ ? 'http://localhost:8321' : null;

// Remote server — online features (PvP, auth, decks, friends, lobby)
const remoteBaseURL = import.meta.env.VITE_API_URL || '';
```

| Feature | Server | Transport |
|---------|--------|-----------|
| Play vs agent | Local sidecar | HTTP REST |
| Deck building/validation | Local sidecar | HTTP REST |
| Login, profile, saved decks | Remote server | HTTP REST |
| PvP matchmaking | Remote server | HTTP REST (lobby API) |
| PvP gameplay | Remote server | WebSocket |
| Spectating | Remote server | WebSocket |

The game API client should route requests based on game mode — local games go to `localhost:8321`, online games go to the remote server. The Zustand game store doesn't need to know the difference.

**Note:** Tauri's webview serves the frontend directly from the built `dist/` folder — no static file serving needed from the Python sidecar.

### 3.10 .gitignore Updates

**Modify: `.gitignore`**

```gitignore
# Tauri
code/src-tauri/target/
code/src-tauri/binaries/
code/src-tauri/resources/models/
*.AppImage
*.dmg
*.msi
*.deb
```

### 3.11 Files Summary

| File | Action | Description |
|------|--------|-------------|
| `code/src-tauri/tauri.conf.json` | Create | Tauri build configuration |
| `code/src-tauri/src/main.rs` | Create | Sidecar lifecycle management |
| `code/src-tauri/Cargo.toml` | Create | Rust dependencies (tauri, tauri-plugin-shell) |
| `code/src-tauri/capabilities/default.json` | Create | Permission config |
| `code/src-tauri/icons/` | Create | App icons (generated by `tauri icon`) |
| `code/digimon_gym/desktop_main.py` | Create | Lightweight sidecar entry point (game engine only, no DB/auth) |
| `desktop.spec` | Create | PyInstaller spec file |
| `scripts/build-sidecar.sh` | Create | Build script for sidecar binary |
| `code/frontend/src/api/client.ts` | Modify | Dual-server routing (local sidecar + remote server) |
| `.gitignore` | Modify | Ignore Tauri build artifacts |

### 3.12 Verification

- `scripts/build-sidecar.sh gameplay` produces a working binary
- Binary starts, serves game API on localhost:8321 (no DB required)
- `cd src-tauri && cargo tauri dev` launches the app with sidecar
- `cargo tauri build` produces platform installers (.dmg, .msi, .AppImage)
- ONNX agent play works through the Tauri app (full build)
- Game vs greedy agent works offline (no internet, no server connection)
- Online features (login, PvP, lobby) route to remote server correctly
- WebSocket PvP works when connected to remote server

---

## Dependency Graph

```
PR 1 (WebSocket PvP)          PR 2 (ONNX Inference)
         │                            │
         │                            │
         └────────┬───────────────────┘
                  │
                  ▼
          PR 3 (Tauri Desktop)
```

PR 1 and PR 2 are independent of each other and could be developed in parallel. PR 3 depends on both (it bundles WebSocket support for remote PvP and ONNX for local agent play).

---

## Build Order

1. **PR 1: WebSocket PvP + Spectating** — highest user-facing value, unblocks testing of network play
2. **PR 2: ONNX Inference** — unblocks gameplay-only desktop build, relatively self-contained
3. **PR 3: Tauri Desktop** — packages everything, depends on both previous PRs
