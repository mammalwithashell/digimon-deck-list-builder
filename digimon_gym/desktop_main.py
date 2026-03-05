"""Desktop sidecar entry point. Lightweight — game engine + deck tools only.

No database, no auth, no admin, no AI pipeline. All online features
(PvP, auth, decks, friends, lobby) go through the central server.

This module creates a minimal FastAPI app that can be bundled with
PyInstaller and run as a Tauri sidecar.
"""

from __future__ import annotations

import argparse
import os

import uvicorn
from fastapi import FastAPI, APIRouter, HTTPException
from fastapi.middleware.cors import CORSMiddleware

from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.data.deck_loader import parse_deck
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

    router = APIRouter(tags=["games"])

    @router.get("/health")
    def health():
        return {"status": "ok", "mode": "desktop"}

    @router.post("/games")
    def create_game(request: CreateGameRequest):
        """Create a new local game session."""
        from pathlib import Path
        from uuid import uuid4

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

        # Resolve ONNX model paths
        models_path = Path(models_dir)
        p1_model = str(models_path / Path(request.player1_model).name) if p1_policy == "trained" and request.player1_model else None
        p2_model = str(models_path / Path(request.player2_model).name) if p2_policy == "trained" and request.player2_model else None

        runner = InteractiveGame(
            deck1, deck2, p1_type, p2_type,
            player1_policy=p1_policy, player2_policy=p2_policy,
            agent_action_delay_ms=request.agent_action_delay_ms,
            player1_model_path=p1_model, player2_model_path=p2_model,
        )
        active_games[game_id] = runner
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
        return runner.game.to_ui_json()

    @router.get("/games/{game_id}/action-mask")
    def game_mask(game_id: str):
        runner = active_games.get(game_id)
        if not runner:
            raise HTTPException(status_code=404, detail="Game not found")
        return {"action_mask": runner.get_action_mask().tolist()}

    @router.get("/games/models")
    def list_models():
        from pathlib import Path
        md = Path(models_dir)
        if not md.exists():
            return {"models": []}
        return {"models": sorted(f.name for f in md.glob("*.onnx"))}

    @router.delete("/games/{game_id}")
    def delete_game(game_id: str):
        if game_id in active_games:
            del active_games[game_id]
        return {"status": "deleted"}

    app.include_router(router)

    # Deck tools have no DB deps — safe to import directly
    from digimon_gym.routers import deck_tools
    app.include_router(deck_tools.router)

    return app


def main():
    parser = argparse.ArgumentParser(description="Digimon TCG Desktop Sidecar")
    parser.add_argument("--port", type=int, default=8321)
    parser.add_argument("--models-dir", default="./models")
    args = parser.parse_args()

    app = create_desktop_app(args.models_dir)
    uvicorn.run(app, host="127.0.0.1", port=args.port)


if __name__ == "__main__":
    main()
