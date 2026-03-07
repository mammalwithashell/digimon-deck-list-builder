"""Replay session endpoints."""

from __future__ import annotations

from uuid import uuid4

from fastapi import APIRouter, HTTPException

from digimon_gym.routers.schemas import (
    ReplayCreateResponse,
    ReplayRequest,
    ReplayStepResponse,
    SeekRequest,
)
from digimon_gym.engine.runners.replay_runner import ReplayRunner
from digimon_gym.routers.state import active_replays

router = APIRouter(tags=["replays"])


def _require_replay(replay_id: str) -> ReplayRunner:
    runner = active_replays.get(replay_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Replay session not found")
    return runner


def _to_step_response(result) -> ReplayStepResponse:
    return ReplayStepResponse(
        step_number=result.step_number,
        action_id=result.action_id,
        player_id=result.player_id,
        phase_before=result.phase_before,
        phase_after=result.phase_after,
        memory_before=result.memory_before,
        memory_after=result.memory_after,
        turn_number=result.turn_number,
        is_game_over=result.is_game_over,
        winner_id=result.winner_id,
        state=result.state,
        verification_ok=result.verification_ok,
        verification_errors=result.verification_errors,
    )


@router.post("/replays", response_model=ReplayCreateResponse)
@router.post("/games/replay", response_model=ReplayCreateResponse, include_in_schema=False)
def create_replay(request: ReplayRequest):
    """Create a replay session from a recording dict."""
    try:
        runner = ReplayRunner(request.recording, verify=request.verify)
    except (ValueError, KeyError) as exc:
        raise HTTPException(status_code=400, detail=f"Invalid recording: {exc}")

    replay_id = str(uuid4())
    active_replays[replay_id] = runner

    return ReplayCreateResponse(
        replay_id=replay_id,
        total_steps=runner.total_steps,
        initial_state=runner.get_state(),
    )


@router.post("/replays/{replay_id}/steps", response_model=ReplayStepResponse)
@router.post("/games/replay/{replay_id}/step", response_model=ReplayStepResponse, include_in_schema=False)
def replay_step(replay_id: str):
    """Advance a replay by one action."""
    runner = _require_replay(replay_id)
    if runner.is_complete:
        raise HTTPException(status_code=400, detail="Replay is complete - no more actions")

    result = runner.step()
    return _to_step_response(result)


@router.post("/replays/{replay_id}/seek", response_model=ReplayStepResponse)
@router.post("/games/replay/{replay_id}/seek", response_model=ReplayStepResponse, include_in_schema=False)
def replay_seek(replay_id: str, request: SeekRequest):
    """Jump to a specific step in a replay."""
    runner = _require_replay(replay_id)
    try:
        result = runner.seek(request.step)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    return _to_step_response(result)


@router.delete("/replays/{replay_id}")
@router.delete("/games/replay/{replay_id}", include_in_schema=False)
def delete_replay(replay_id: str):
    """Destroy a replay session."""
    if replay_id in active_replays:
        del active_replays[replay_id]
    return {"status": "deleted"}

