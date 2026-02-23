"""Recording and bug-report endpoints."""

from __future__ import annotations

import json

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.database import get_db
from digimon_gym.db.models import GameRecording, GameRecordingReport
from digimon_gym.db.schemas import BugReportRequest, BugReportResponse, RecordingResponse
from digimon_gym.engine.runners.replay_runner import ReplayRunner

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
        runner = ReplayRunner(recording_data)
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
        replay_result = runner.seek(step)
    except (ValueError, IndexError) as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    return replay_result.state


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

