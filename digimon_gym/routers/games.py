"""Game session endpoints."""

from __future__ import annotations

import json
from uuid import uuid4

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.database import get_db
from digimon_gym.db.models import GameRecording
from digimon_gym.db.schemas import RecordingSaveResponse
from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.data.deck_loader import parse_deck
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.routers.schemas import CreateGameRequest, GameActionRequest
from digimon_gym.routers.state import active_games

router = APIRouter(tags=["games"])


def _require_game(game_id: str):
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    return runner


@router.post("/games")
@router.post("/game/create", include_in_schema=False)
def create_game(request: CreateGameRequest):
    """Create a new game session and return its initial state."""
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

    if p1_type == PlayerType.Agent and p2_type == PlayerType.Agent:
        runner = HeadlessGame(
            deck1,
            deck2,
            verbose=True,
            record_actions=request.record_actions,
            record_tensors=request.record_tensors,
        )
    else:
        runner = InteractiveGame(
            deck1,
            deck2,
            p1_type,
            p2_type,
            player1_policy=p1_policy,
            player2_policy=p2_policy,
        )

    active_games[game_id] = runner

    if isinstance(runner, InteractiveGame):
        runner.clear_log()

    state = runner.game.to_ui_json()
    mask = runner.get_action_mask().tolist()

    player_labels = {
        1: "You" if p1_type == PlayerType.Human else "Agent",
        2: "You" if p2_type == PlayerType.Human else "Agent",
    }

    result = {
        "game_id": game_id,
        "state": state,
        "action_mask": mask,
        "action_descriptions": runner.game.describe_actions(runner.game.current_player_id),
        "player_labels": player_labels,
    }

    if isinstance(runner, InteractiveGame):
        result["recording_metadata"] = runner.get_initial_state_dict()

    return result


@router.post("/games/{game_id}/actions")
@router.post("/game/{game_id}/action", include_in_schema=False)
def game_action(game_id: str, request: GameActionRequest):
    """Execute a single action in an active game."""
    runner = _require_game(game_id)

    current_player_id = runner.game.current_player_id
    memory_before = runner.game.memory
    phase_before = runner.game.current_phase.name
    turn_before = runner.game.turn_count

    runner.step(request.action)
    state = runner.game.to_ui_json()
    mask = runner.get_action_mask().tolist()

    result = {
        "state": state,
        "action_mask": mask,
        "action_descriptions": runner.game.describe_actions(runner.game.current_player_id),
        "is_game_over": runner.is_game_over,
        "action_context": {
            "player_id": current_player_id,
            "action_id": request.action,
            "phase": phase_before,
            "memory_before": memory_before,
            "memory_after": runner.game.memory,
            "turn": turn_before,
        },
    }

    if isinstance(runner, InteractiveGame):
        result["logs"] = runner.get_last_log()
        runner.clear_log()

    return result


@router.post("/games/{game_id}/steps")
@router.post("/game/{game_id}/step", include_in_schema=False)
def game_step(game_id: str):
    """Advance an interactive game until it is a human turn or game over."""
    runner = _require_game(game_id)
    if not isinstance(runner, InteractiveGame):
        raise HTTPException(status_code=400, detail="Step is only for interactive games. Use /action for headless games.")

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
@router.get("/game/{game_id}/state", include_in_schema=False)
def game_state(game_id: str):
    """Get current game state."""
    runner = _require_game(game_id)
    return runner.game.to_ui_json()


@router.get("/games/{game_id}/action-mask")
@router.get("/game/{game_id}/mask", include_in_schema=False)
def game_mask(game_id: str):
    """Get current action mask."""
    runner = _require_game(game_id)
    return {"action_mask": runner.get_action_mask().tolist()}


@router.get("/games/{game_id}/actions")
@router.get("/game/{game_id}/actions", include_in_schema=False)
def game_actions(game_id: str):
    """Get human-readable descriptions of currently legal actions."""
    runner = _require_game(game_id)
    return {
        "actions": runner.game.describe_actions(runner.game.current_player_id),
    }


@router.get("/games/{game_id}/logs")
@router.get("/game/{game_id}/log", include_in_schema=False)
def game_log(game_id: str):
    """Get and clear game logs."""
    runner = _require_game(game_id)
    if isinstance(runner, InteractiveGame):
        logs = runner.get_last_log()
        runner.clear_log()
        return {"logs": logs}
    return {"logs": []}


@router.delete("/games/{game_id}")
@router.delete("/game/{game_id}", include_in_schema=False)
def delete_game(game_id: str):
    """Delete an active game session."""
    if game_id in active_games:
        del active_games[game_id]
    return {"status": "deleted"}


@router.get("/games/{game_id}/recording")
@router.get("/game/{game_id}/recording", include_in_schema=False)
def get_game_recording(game_id: str):
    """Get in-memory recording for a headless game."""
    runner = _require_game(game_id)
    if not isinstance(runner, HeadlessGame):
        raise HTTPException(
            status_code=400,
            detail="Recording endpoint is for headless games. Interactive games use client-side recording.",
        )

    recording = runner.get_recording()
    if not recording:
        raise HTTPException(
            status_code=404,
            detail="This game was not created with recording enabled (record_actions=True).",
        )
    return recording


@router.post("/games/{game_id}/recordings", response_model=RecordingSaveResponse)
@router.post("/game/{game_id}/recording/save", response_model=RecordingSaveResponse, include_in_schema=False)
async def save_game_recording(game_id: str, db: AsyncSession = Depends(get_db)):
    """Persist a headless game recording to the database."""
    runner = _require_game(game_id)
    if not isinstance(runner, HeadlessGame):
        raise HTTPException(status_code=400, detail="Only headless game recordings can be saved server-side.")

    recording_data = runner.get_recording()
    if not recording_data:
        raise HTTPException(status_code=404, detail="No recording data available.")

    record = GameRecording(
        game_mode="headless",
        recording_json=json.dumps(recording_data),
        total_steps=recording_data.get("total_actions", 0),
        has_tensors=1 if recording_data.get("tensor_snapshots_count", 0) > 0 else 0,
    )
    db.add(record)
    await db.commit()
    await db.refresh(record)

    return RecordingSaveResponse(recording_id=record.id)
