"""Desktop sidecar entry point. Lightweight — game engine + deck tools only.

No database, no auth, no admin, no AI pipeline. All online features
(PvP, auth, decks, friends, lobby) go through the central server.

This module creates a minimal FastAPI app that can be bundled with
PyInstaller and run as a Tauri sidecar.
"""

from __future__ import annotations

import argparse
import os
import socket
import time

import uvicorn
from fastapi import FastAPI, APIRouter, HTTPException
from fastapi.middleware.cors import CORSMiddleware

from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.data.deck_loader import parse_deck
from digimon_gym.engine.model_utils import resolve_model_path, list_onnx_models
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.routers.schemas import CreateGameRequest, GameActionRequest


def create_desktop_app(models_dir: str = "./models") -> FastAPI:
    """Create a minimal FastAPI app for desktop sidecar use.

    Only mounts game engine and deck tool routes — no DB, no auth.
    """
    os.environ["ONNX_MODELS_DIR"] = models_dir

    app = FastAPI(title="Digimon TCG Desktop", version="0.1.0")
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],
        allow_methods=["*"],
        allow_headers=["*"],
    )

    # In-memory game storage (no persistence needed for desktop)
    active_games: dict[str, InteractiveGame] = {}
    game_last_access: dict[str, float] = {}

    MAX_GAMES = 10
    GAME_TTL_SECONDS = 3600  # 1 hour

    def _cleanup_stale_games():
        """Remove games that haven't been accessed within TTL."""
        now = time.time()
        expired = [gid for gid, ts in game_last_access.items()
                   if now - ts > GAME_TTL_SECONDS]
        for gid in expired:
            active_games.pop(gid, None)
            game_last_access.pop(gid, None)

    def _touch_game(game_id: str):
        """Update last-access timestamp for a game."""
        game_last_access[game_id] = time.time()

    router = APIRouter(tags=["games"])

    @router.get("/health")
    def health():
        return {"status": "ok", "mode": "desktop"}

    @router.post("/games")
    def create_game(request: CreateGameRequest):
        """Create a new local game session."""
        from uuid import uuid4

        _cleanup_stale_games()
        # Evict oldest game if at capacity
        while len(active_games) >= MAX_GAMES and game_last_access:
            oldest = min(game_last_access, key=game_last_access.get)
            active_games.pop(oldest, None)
            game_last_access.pop(oldest, None)

        try:
            deck1 = parse_deck(request.deck1_raw) if request.deck1_raw else request.deck1
            deck2 = parse_deck(request.deck2_raw) if request.deck2_raw else request.deck2
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=f"Deck parsing error: {exc}")

        if not deck1 or not deck2:
            raise HTTPException(status_code=400, detail="Both decks must be provided")

        game_id = str(uuid4())
        p1_type = PlayerType.Human if request.player1_type.lower() == "human" else PlayerType.Agent
        p2_type = PlayerType.Human if request.player2_type.lower() == "human" else PlayerType.Agent
        p1_policy = request.player1_policy.lower()
        p2_policy = request.player2_policy.lower()

        # Resolve ONNX model paths (shared helper handles sanitization)
        try:
            p1_model = resolve_model_path(request.player1_model) if p1_policy == "trained" else None
            p2_model = resolve_model_path(request.player2_model) if p2_policy == "trained" else None
        except FileNotFoundError as exc:
            raise HTTPException(status_code=400, detail=str(exc))

        runner = InteractiveGame(
            deck1, deck2, p1_type, p2_type,
            player1_policy=p1_policy, player2_policy=p2_policy,
            agent_action_delay_ms=request.agent_action_delay_ms,
            player1_model_path=p1_model, player2_model_path=p2_model,
        )
        active_games[game_id] = runner
        _touch_game(game_id)
        runner.clear_log()

        state = runner.game.to_ui_json()
        mask = runner.get_action_mask().tolist()
        player_labels = {
            1: "You" if p1_type == PlayerType.Human else "Agent",
            2: "You" if p2_type == PlayerType.Human else "Agent",
        }
        return {
            "game_id": game_id,
            "state": state,
            "action_mask": mask,
            "action_descriptions": runner.game.describe_actions(runner.game.current_player_id),
            "player_labels": player_labels,
            "recording_metadata": runner.get_initial_state_dict(),
        }

    @router.post("/games/{game_id}/actions")
    def game_action(game_id: str, request: GameActionRequest):
        runner = active_games.get(game_id)
        if not runner:
            raise HTTPException(status_code=404, detail="Game not found")
        _touch_game(game_id)

        current_player_id = runner.game.current_player_id
        memory_before = runner.game.memory
        phase_before = runner.game.current_phase.name
        turn_before = runner.game.turn_count

        runner.step(request.action)
        state = runner.game.to_ui_json()
        mask = runner.get_action_mask().tolist()
        logs = runner.get_last_log()
        runner.clear_log()

        return {
            "state": state,
            "action_mask": mask,
            "action_descriptions": runner.game.describe_actions(runner.game.current_player_id),
            "is_game_over": runner.is_game_over,
            "logs": logs,
            "action_context": {
                "player_id": current_player_id,
                "action_id": request.action,
                "phase": phase_before,
                "memory_before": memory_before,
                "memory_after": runner.game.memory,
                "turn": turn_before,
            },
        }

    @router.post("/games/{game_id}/steps")
    def game_step(game_id: str):
        runner = active_games.get(game_id)
        if not runner:
            raise HTTPException(status_code=404, detail="Game not found")
        _touch_game(game_id)

        state = runner.run_step()
        mask = runner.get_action_mask().tolist()
        logs = runner.get_last_log()
        runner.clear_log()

        return {
            "state": state,
            "action_mask": mask,
            "action_descriptions": runner.game.describe_actions(runner.game.current_player_id),
            "logs": logs,
            "is_human_turn": runner.is_current_player_human(),
            "is_game_over": runner.is_game_over,
        }

    @router.get("/games/{game_id}/state")
    def game_state(game_id: str):
        runner = active_games.get(game_id)
        if not runner:
            raise HTTPException(status_code=404, detail="Game not found")
        _touch_game(game_id)
        return runner.game.to_ui_json()

    @router.get("/games/{game_id}/action-mask")
    def game_mask(game_id: str):
        runner = active_games.get(game_id)
        if not runner:
            raise HTTPException(status_code=404, detail="Game not found")
        _touch_game(game_id)
        return {"action_mask": runner.get_action_mask().tolist()}

    @router.get("/games/{game_id}/logs")
    def game_logs(game_id: str):
        runner = active_games.get(game_id)
        if not runner:
            raise HTTPException(status_code=404, detail="Game not found")
        _touch_game(game_id)
        if isinstance(runner, InteractiveGame):
            logs = runner.get_last_log()
            runner.clear_log()
            return {"logs": logs}
        return {"logs": []}

    @router.get("/games/models")
    def list_models():
        return {"models": list_onnx_models()}

    @router.delete("/games/{game_id}")
    def delete_game(game_id: str):
        active_games.pop(game_id, None)
        game_last_access.pop(game_id, None)
        return {"status": "deleted"}

    app.include_router(router)

    # Deck tools have no DB deps — safe to import directly
    from digimon_gym.routers import deck_tools
    app.include_router(deck_tools.router)

    return app


def _find_available_port(start: int = 8321, max_attempts: int = 10) -> int:
    """Find an available port starting from *start*, trying up to *max_attempts*."""
    for offset in range(max_attempts):
        port = start + offset
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            try:
                s.bind(("127.0.0.1", port))
                return port
            except OSError:
                continue
    raise RuntimeError(
        f"No available port in range {start}-{start + max_attempts - 1}"
    )


def main():
    parser = argparse.ArgumentParser(description="Digimon TCG Desktop Sidecar")
    parser.add_argument("--port", type=int, default=8321)
    parser.add_argument("--models-dir", default="./models")
    args = parser.parse_args()

    port = _find_available_port(args.port)
    # Print port for Tauri to discover via stdout parsing
    print(f"SIDECAR_PORT={port}", flush=True)

    app = create_desktop_app(args.models_dir)
    uvicorn.run(app, host="127.0.0.1", port=port)


if __name__ == "__main__":
    main()
