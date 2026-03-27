"""Game session endpoints.

Engine-only router — no database or auth dependencies.
Safe to mount in the desktop sidecar alongside the hosted API.
"""

from __future__ import annotations

from fastapi import APIRouter, HTTPException

from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.data.deck_loader import parse_deck
from digimon_gym.engine.model_utils import list_onnx_models, resolve_model_path
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.routers.schemas import CreateGameRequest, GameActionRequest, SurrenderRequest
from digimon_gym.routers.state import game_service

router = APIRouter(tags=["games"])


def _resolve_model_path(model_name: str | None) -> str | None:
    """Resolve an ONNX model filename, raising HTTPException on failure."""
    try:
        return resolve_model_path(model_name)
    except FileNotFoundError as exc:
        raise HTTPException(status_code=400, detail=str(exc))


def _require_game(game_id: str):
    runner = game_service.get(game_id)
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

    p1_type = PlayerType.Human if request.player1_type.lower() == "human" else PlayerType.Agent
    p2_type = PlayerType.Human if request.player2_type.lower() == "human" else PlayerType.Agent
    p1_policy = request.player1_policy.lower()
    p2_policy = request.player2_policy.lower()

    # Resolve ONNX model paths for "trained" policies
    p1_model_path = _resolve_model_path(request.player1_model) if p1_policy == "trained" else None
    p2_model_path = _resolve_model_path(request.player2_model) if p2_policy == "trained" else None

    if p1_type == PlayerType.Agent and p2_type == PlayerType.Agent:
        game_id, runner = game_service.create_headless_game(
            deck1, deck2,
            verbose=True,
            record_actions=request.record_actions,
            record_tensors=request.record_tensors,
        )
    else:
        game_id, runner = game_service.create_interactive_game(
            deck1, deck2, p1_type, p2_type,
            p1_policy=p1_policy,
            p2_policy=p2_policy,
            agent_action_delay_ms=request.agent_action_delay_ms,
            p1_model_path=p1_model_path,
            p2_model_path=p2_model_path,
        )

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
        result["events"] = runner.get_last_events()
        runner.clear_log()
        runner.clear_events()

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
    events = runner.get_last_events()
    runner.clear_log()
    runner.clear_events()

    return {
        "state": state,
        "action_mask": mask,
        "action_descriptions": runner.game.describe_actions(runner.game.current_player_id),
        "logs": logs,
        "events": events,
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


@router.post("/games/{game_id}/surrender")
def surrender_game(game_id: str, request: SurrenderRequest):
    """Surrender an active game."""
    runner = _require_game(game_id)
    if not isinstance(runner, InteractiveGame):
        raise HTTPException(status_code=400, detail="Surrender is only for interactive games.")
    if runner.game.game_over:
        raise HTTPException(status_code=400, detail="Game is already over.")

    state = runner.surrender(request.player_id)
    logs = runner.get_last_log()
    events = runner.get_last_events()
    runner.clear_log()
    runner.clear_events()

    return {
        "state": state,
        "action_mask": runner.get_action_mask().tolist(),
        "logs": logs,
        "events": events,
        "is_game_over": True,
        "surrendered_by": request.player_id,
    }


@router.delete("/games/{game_id}")
@router.delete("/game/{game_id}", include_in_schema=False)
def delete_game(game_id: str):
    """Delete an active game session."""
    game_service.delete(game_id)
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



@router.get("/games/models")
def list_available_models():
    """List available ONNX agent models."""
    return {"models": list_onnx_models()}
