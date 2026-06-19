"""Recording and bug-report endpoints."""

from __future__ import annotations

import json

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from server.db.database import get_db
from server.db.models import GameRecording, GameRecordingReport
from server.db.schemas import BugReportRequest, BugReportResponse, RecordingResponse, RecordingSaveResponse
from digimon_engine import RustHeadlessGame, RustReplayRunner
from server.routers.state import active_games

router = APIRouter(tags=["recordings"])


@router.get("/recordings")
@router.get("/games/recordings", include_in_schema=False)
async def list_recordings(db: AsyncSession = Depends(get_db)):
    """List saved game recordings from the database."""
    result = await db.execute(select(GameRecording).order_by(GameRecording.created_at.desc()))
    recordings = result.scalars().all()
    return [
        RecordingResponse(
            id=recording.id,
            game_mode=recording.game_mode,
            total_steps=recording.total_steps,
            has_tensors=bool(recording.has_tensors),
            created_at=recording.created_at,
        )
        for recording in recordings
    ]


@router.get("/recordings/{recording_id}")
@router.get("/games/recordings/{recording_id}", include_in_schema=False)
async def get_saved_recording(recording_id: str, db: AsyncSession = Depends(get_db)):
    """Get a saved recording by id."""
    result = await db.execute(select(GameRecording).where(GameRecording.id == recording_id))
    record = result.scalar_one_or_none()
    if not record:
        raise HTTPException(status_code=404, detail="Recording not found")
    return json.loads(record.recording_json)


@router.get("/recordings/{recording_id}/state")
@router.get("/games/recordings/{recording_id}/state", include_in_schema=False)
async def get_recording_state_at_step(
    recording_id: str,
    step: int = Query(0, ge=0),
    db: AsyncSession = Depends(get_db),
):
    """Stateless replay: load recording and return game state at a specific step."""
    result = await db.execute(select(GameRecording).where(GameRecording.id == recording_id))
    record = result.scalar_one_or_none()
    if not record:
        raise HTTPException(status_code=404, detail="Recording not found")

    recording_data = json.loads(record.recording_json)

    try:
        runner = RustReplayRunner(json.dumps(recording_data))
    except (ValueError, KeyError) as exc:
        raise HTTPException(status_code=500, detail=f"Failed to load recording: {exc}")

    if step == 0:
        return runner.get_state()

    if step > runner.total_steps:
        raise HTTPException(
            status_code=400,
            detail=f"step must be 0..{runner.total_steps}, got {step}",
        )

    try:
        runner.seek(step)
    except (ValueError, IndexError) as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    return runner.get_state()


@router.post("/games/{game_id}/recordings", response_model=RecordingSaveResponse)
@router.post("/game/{game_id}/recording/save", response_model=RecordingSaveResponse, include_in_schema=False)
async def save_game_recording(game_id: str, db: AsyncSession = Depends(get_db)):
    """Persist a headless game recording to the database."""
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    if not isinstance(runner, RustHeadlessGame):
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


@router.post("/bug-reports", response_model=BugReportResponse)
@router.post("/games/report", response_model=BugReportResponse, include_in_schema=False)
async def submit_bug_report(request: BugReportRequest, db: AsyncSession = Depends(get_db)):
    """Persist a client-side recording bundle for bug triage."""
    report = GameRecordingReport(
        description=request.description,
        recording_json=json.dumps(request.recording),
    )
    db.add(report)
    await db.commit()
    await db.refresh(report)
    return BugReportResponse(report_id=report.id)

