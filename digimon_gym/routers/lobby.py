"""Game lobby for network PvP matchmaking.

Players create lobbies (with a shareable join code) or browse public games.
Once two players are matched, an ``InteractiveGame`` is created and both
players connect via WebSocket to play.
"""

from __future__ import annotations

import logging
import secrets
import string
from datetime import datetime, timezone
from typing import Optional
from uuid import uuid4

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.auth import get_current_user
from digimon_gym.db.database import get_db
from digimon_gym.db.models import User
from digimon_gym.engine.data.deck_loader import parse_deck
from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.routers.state import active_games
from digimon_gym.routers.ws_manager import GameSettings, manager

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/lobby", tags=["lobby"])

# ── In-memory lobby state ────────────────────────────────────────────────

_CODE_CHARS = string.ascii_uppercase + string.digits


def _generate_join_code(length: int = 6) -> str:
    return "".join(secrets.choice(_CODE_CHARS) for _ in range(length))


class PendingGame(BaseModel):
    game_id: str
    join_code: str
    host_user_id: str
    host_display_name: str
    host_deck: list[str]
    host_deck_raw: Optional[str] = None
    created_at: datetime
    is_public: bool = True
    allow_spectators: bool = True
    spectator_mode: str = "hidden"


# game_id → PendingGame
pending_games: dict[str, PendingGame] = {}
# join_code → game_id (for fast lookup)
code_to_game: dict[str, str] = {}


# ── Request / Response Schemas ───────────────────────────────────────────

class CreateLobbyRequest(BaseModel):
    deck: list[str] = Field(default_factory=list)
    deck_raw: Optional[str] = None
    is_public: bool = True
    allow_spectators: bool = True
    spectator_mode: str = Field("hidden", pattern="^(hidden|open)$")


class JoinLobbyRequest(BaseModel):
    deck: list[str] = Field(default_factory=list)
    deck_raw: Optional[str] = None


# ── Endpoints ────────────────────────────────────────────────────────────

@router.post("/create")
async def create_lobby_game(
    request: CreateLobbyRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host creates a game and gets a join code.  Game is not started yet."""
    # Validate deck
    try:
        deck = parse_deck(request.deck_raw) if request.deck_raw else request.deck
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=f"Deck parsing error: {exc}")
    if not deck:
        raise HTTPException(status_code=400, detail="A deck must be provided")

    game_id = str(uuid4())
    join_code = _generate_join_code()

    # Ensure code uniqueness (extremely unlikely collision)
    while join_code in code_to_game:
        join_code = _generate_join_code()

    pending = PendingGame(
        game_id=game_id,
        join_code=join_code,
        host_user_id=user.id,
        host_display_name=user.display_name or user.username,
        host_deck=deck,
        host_deck_raw=request.deck_raw,
        created_at=datetime.now(timezone.utc),
        is_public=request.is_public,
        allow_spectators=request.allow_spectators,
        spectator_mode=request.spectator_mode,
    )

    pending_games[game_id] = pending
    code_to_game[join_code] = game_id

    # Pre-register settings in the connection manager
    manager.set_settings(game_id, GameSettings(
        allow_spectators=request.allow_spectators,
        spectator_mode=request.spectator_mode,
        host_user_id=user.id,
    ))

    logger.info("Lobby created: game_id=%s code=%s host=%s", game_id, join_code, user.username)

    return {
        "game_id": game_id,
        "join_code": join_code,
    }


@router.post("/join/{join_code}")
async def join_lobby_game(
    join_code: str,
    request: JoinLobbyRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Second player joins via code.  Creates the InteractiveGame and returns game_id."""
    join_code = join_code.upper().strip()
    game_id = code_to_game.get(join_code)
    if game_id is None or game_id not in pending_games:
        raise HTTPException(status_code=404, detail="Game not found or already started")

    pending = pending_games[game_id]

    # Can't join your own game
    if pending.host_user_id == user.id:
        raise HTTPException(status_code=400, detail="Cannot join your own game")

    # Validate joiner's deck
    try:
        deck2 = parse_deck(request.deck_raw) if request.deck_raw else request.deck
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=f"Deck parsing error: {exc}")
    if not deck2:
        raise HTTPException(status_code=400, detail="A deck must be provided")

    # Create the InteractiveGame (both players are human)
    runner = InteractiveGame(
        pending.host_deck,
        deck2,
        player1_type=PlayerType.Human,
        player2_type=PlayerType.Human,
        agent_action_delay_ms=0,
    )

    active_games[game_id] = runner

    # Clean up lobby state
    del pending_games[game_id]
    del code_to_game[join_code]

    logger.info(
        "Player %s joined game %s (host: %s)",
        user.username, game_id, pending.host_display_name,
    )

    return {
        "game_id": game_id,
        "player_id": 2,
    }


@router.get("/games")
async def list_lobby_games() -> list[dict]:
    """List public pending games (for lobby browser)."""
    return [
        {
            "game_id": pg.game_id,
            "join_code": pg.join_code,
            "host_display_name": pg.host_display_name,
            "created_at": pg.created_at.isoformat(),
            "allow_spectators": pg.allow_spectators,
        }
        for pg in pending_games.values()
        if pg.is_public
    ]


@router.delete("/{game_id}")
async def cancel_lobby_game(
    game_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host cancels a pending game."""
    pending = pending_games.get(game_id)
    if pending is None:
        raise HTTPException(status_code=404, detail="Pending game not found")
    if pending.host_user_id != user.id:
        raise HTTPException(status_code=403, detail="Only the host can cancel")

    code_to_game.pop(pending.join_code, None)
    del pending_games[game_id]
    manager.cleanup_game(game_id)

    return {"status": "cancelled"}
