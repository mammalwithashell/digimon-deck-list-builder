"""Replay session endpoints."""

from __future__ import annotations

import json
from uuid import uuid4

from fastapi import APIRouter, HTTPException

from server.routers.schemas import (
    ReplayCreateResponse,
    ReplayRequest,
    ReplayStepResponse,
    SeekRequest,
)
from digimon_engine import RustReplayRunner
from server.routers.state import active_replays

router = APIRouter(tags=["replays"])


def _require_replay(replay_id: str):
    runner = active_replays.get(replay_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Replay session not found")
    return runner


def _to_step_response(result: dict) -> ReplayStepResponse:
    """Map a `RustReplayRunner.step()` dict into the API step response."""
    divergences = result.get("divergences", [])
    return ReplayStepResponse(
        step_number=result["step_number"],
        action_id=result["action_id"],
        player_id=result["player_id"],
        phase_before=result["phase_before"],
        phase_after=result["phase_after"],
        memory_before=result["memory_before"],
        memory_after=result["memory_after"],
        turn_number=result["turn_number"],
        is_game_over=result["is_game_over"],
        winner_id=result["winner_id"],
        state=result["state"],
        verification_ok=not divergences,
        verification_errors=[
            f"step {d['step']} {d['field']}: recorded={d['recorded']} replayed={d['replayed']}"
            for d in divergences
        ],
    )


def _seek_response(runner) -> ReplayStepResponse:
    """Build a step response for the current (post-seek) replay position.

    Seek positions the cursor without applying a new action, so action-specific
    fields are read from the current board (`get_state`).
    """
    state = runner.get_state()
    return ReplayStepResponse(
        step_number=runner.current_step,
        action_id=0,
        player_id=state.get("currentPlayer", 0),
        phase_before=state.get("currentPhase", ""),
        phase_after=state.get("currentPhase", ""),
        memory_before=state.get("memoryGauge", 0),
        memory_after=state.get("memoryGauge", 0),
        turn_number=state.get("turnCount", 0),
        is_game_over=runner.is_game_over,
        winner_id=runner.winner_id,
        state=state,
        verification_ok=True,
        verification_errors=[],
    )


@router.post("/replays", response_model=ReplayCreateResponse)
@router.post("/games/replay", response_model=ReplayCreateResponse, include_in_schema=False)
def create_replay(request: ReplayRequest):
    """Create a replay session from a recording dict."""
    try:
        runner = RustReplayRunner(json.dumps(request.recording), verify=request.verify)
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
        runner.seek(request.step)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    return _seek_response(runner)


@router.delete("/replays/{replay_id}")
@router.delete("/games/replay/{replay_id}", include_in_schema=False)
def delete_replay(replay_id: str):
    """Destroy a replay session."""
    if replay_id in active_replays:
        del active_replays[replay_id]
    return {"status": "deleted"}

