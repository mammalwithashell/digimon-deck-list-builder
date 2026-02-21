"""Game history router: session tracking, participant results, user game history."""

from __future__ import annotations

from datetime import datetime, timezone
from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy import select, func
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload

from digimon_gym.db.auth import get_current_user, get_optional_user
from digimon_gym.db.database import get_db
from digimon_gym.db.models import GameParticipant, GameSession, User
from digimon_gym.db.schemas import GameParticipantResponse, GameSessionResponse

router = APIRouter(prefix="/games", tags=["games"])


# ── Response schemas with participants included ──────────────────────────

from pydantic import BaseModel


class GameSessionDetailResponse(GameSessionResponse):
    participants: List[GameParticipantResponse] = []


class GameHistoryEntry(BaseModel):
    """A game session with the requesting user's result highlighted."""
    session: GameSessionResponse
    player_slot: int
    result: Optional[str] = None
    opponent_name: Optional[str] = None


class GameStatsResponse(BaseModel):
    """Aggregate win/loss/draw statistics for a user."""
    total_games: int = 0
    wins: int = 0
    losses: int = 0
    draws: int = 0
    win_rate: float = 0.0


# ── Endpoints ─────────────────────────────────────────────────────────────

@router.get("/history", response_model=List[GameHistoryEntry])
async def get_my_game_history(
    limit: int = Query(20, ge=1, le=100),
    offset: int = Query(0, ge=0),
    game_mode: Optional[str] = Query(None, pattern=r"^(standard|edh_commander|titan|no_restriction)$"),
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """List the authenticated user's game history, most recent first."""
    query = (
        select(GameSession)
        .join(GameParticipant, GameParticipant.game_id == GameSession.id)
        .where(GameParticipant.user_id == user.id)
        .options(selectinload(GameSession.participants))
        .order_by(GameSession.started_at.desc())
        .offset(offset)
        .limit(limit)
    )
    if game_mode:
        query = query.where(GameSession.game_mode == game_mode)

    result = await db.execute(query)
    sessions = result.scalars().unique().all()

    entries = []
    for session in sessions:
        # Find the requesting user's participant record
        my_participant = next(
            (p for p in session.participants if p.user_id == user.id), None
        )
        # Find opponent name (for 2-player games)
        opponent_name = None
        for p in session.participants:
            if p.user_id and p.user_id != user.id:
                opp_result = await db.execute(select(User.display_name, User.username).where(User.id == p.user_id))
                opp = opp_result.one_or_none()
                if opp:
                    opponent_name = opp[0] or opp[1]
                break

        entries.append(GameHistoryEntry(
            session=GameSessionResponse.model_validate(session),
            player_slot=my_participant.player_slot if my_participant else 0,
            result=my_participant.result if my_participant else None,
            opponent_name=opponent_name,
        ))
    return entries


@router.get("/history/stats", response_model=GameStatsResponse)
async def get_my_game_stats(
    game_mode: Optional[str] = Query(None, pattern=r"^(standard|edh_commander|titan|no_restriction)$"),
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """Get aggregate win/loss/draw statistics for the authenticated user."""
    query = select(GameParticipant.result, func.count()).where(
        GameParticipant.user_id == user.id,
        GameParticipant.result.isnot(None),
    ).group_by(GameParticipant.result)

    if game_mode:
        query = query.join(GameSession, GameSession.id == GameParticipant.game_id).where(
            GameSession.game_mode == game_mode
        )

    result = await db.execute(query)
    counts = {row[0]: row[1] for row in result.all()}

    wins = counts.get("win", 0)
    losses = counts.get("loss", 0)
    draws = counts.get("draw", 0)
    total = wins + losses + draws

    return GameStatsResponse(
        total_games=total,
        wins=wins,
        losses=losses,
        draws=draws,
        win_rate=round(wins / total, 4) if total > 0 else 0.0,
    )


@router.get("/sessions/{session_id}", response_model=GameSessionDetailResponse)
async def get_game_session(
    session_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """Get details of a specific game session. User must be a participant."""
    result = await db.execute(
        select(GameSession)
        .where(GameSession.id == session_id)
        .options(selectinload(GameSession.participants))
    )
    session = result.scalar_one_or_none()
    if not session:
        raise HTTPException(status_code=404, detail="Game session not found")

    # Verify user was a participant
    is_participant = any(p.user_id == user.id for p in session.participants)
    if not is_participant:
        raise HTTPException(status_code=403, detail="You are not a participant in this game")

    return GameSessionDetailResponse(
        **GameSessionResponse.model_validate(session).model_dump(),
        participants=[
            GameParticipantResponse.model_validate(p) for p in session.participants
        ],
    )


@router.post("/sessions", response_model=GameSessionDetailResponse, status_code=201)
async def create_game_session(
    game_mode: str = Query(..., pattern=r"^(standard|edh_commander|titan|no_restriction)$"),
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """Create a new game session record. Used when starting an authenticated game."""
    session = GameSession(game_mode=game_mode)
    db.add(session)
    await db.flush()

    # Add the authenticated user as player slot 0
    participant = GameParticipant(
        game_id=session.id,
        player_slot=0,
        user_id=user.id,
        player_type="human",
    )
    db.add(participant)
    await db.commit()
    await db.refresh(session)

    # Re-fetch with participants loaded
    result = await db.execute(
        select(GameSession)
        .where(GameSession.id == session.id)
        .options(selectinload(GameSession.participants))
    )
    session = result.scalar_one()

    return GameSessionDetailResponse(
        **GameSessionResponse.model_validate(session).model_dump(),
        participants=[
            GameParticipantResponse.model_validate(p) for p in session.participants
        ],
    )


@router.post("/sessions/{session_id}/complete")
async def complete_game_session(
    session_id: str,
    winner_slot: Optional[int] = Query(None),
    result_type: str = Query(..., pattern=r"^(completed|concession|timeout|deck_out|abandoned)$"),
    total_turns: Optional[int] = Query(None, ge=0),
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """Mark a game session as complete with results."""
    result = await db.execute(
        select(GameSession)
        .where(GameSession.id == session_id)
        .options(selectinload(GameSession.participants))
    )
    session = result.scalar_one_or_none()
    if not session:
        raise HTTPException(status_code=404, detail="Game session not found")

    is_participant = any(p.user_id == user.id for p in session.participants)
    if not is_participant:
        raise HTTPException(status_code=403, detail="You are not a participant in this game")

    if session.ended_at is not None:
        raise HTTPException(status_code=400, detail="Game session already completed")

    session.ended_at = datetime.now(timezone.utc)
    session.result_type = result_type
    if total_turns is not None:
        session.total_turns = total_turns

    # Set participant results based on winner_slot
    if winner_slot is not None:
        for p in session.participants:
            if p.player_slot == winner_slot:
                p.result = "win"
                session.winner_id = p.user_id
            else:
                p.result = "loss"
    else:
        # No winner (draw or abandoned)
        for p in session.participants:
            p.result = "draw" if result_type != "abandoned" else None

    await db.commit()
    return {"status": "completed", "session_id": session_id}
