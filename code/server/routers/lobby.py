"""Game lobby for network PvP matchmaking.

Players create lobbies (with a shareable join code) or browse public games.
Once two players are matched, an ``InteractiveGame`` is created and both
players connect via WebSocket to play.
"""

from __future__ import annotations

import logging
import secrets
import string
from datetime import datetime, timedelta, timezone
from typing import Optional
from uuid import uuid4

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field
from sqlalchemy.ext.asyncio import AsyncSession

from server.db.auth import get_current_user
from server.db.database import get_db
from server.db.models import User
from digimon_engine import parse_deck
from server.rust_interactive_game import RustInteractiveGame
from server.routers.state import active_games
from server.routers.ws_manager import GameSettings, manager

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
    host_deck: list[str] = Field(default_factory=list)
    host_deck_raw: Optional[str] = None
    created_at: datetime
    is_public: bool = True
    allow_spectators: bool = True
    spectator_mode: str = "hidden"


# game_id → PendingGame
pending_games: dict[str, PendingGame] = {}
# join_code → game_id (for fast lookup)
code_to_game: dict[str, str] = {}

# Pending lobbies older than this are automatically pruned
_LOBBY_TTL = timedelta(minutes=30)


def _prune_stale_lobbies() -> None:
    """Remove pending lobbies that have exceeded the TTL."""
    cutoff = datetime.now(timezone.utc) - _LOBBY_TTL
    expired = [gid for gid, pg in pending_games.items() if pg.created_at < cutoff]
    for gid in expired:
        pg = pending_games.pop(gid)
        code_to_game.pop(pg.join_code, None)
        manager.cleanup_game(gid)
    if expired:
        logger.info("Pruned %d stale lobbies", len(expired))


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


class SetLobbyDeckRequest(BaseModel):
    deck: list[str] = Field(default_factory=list)
    deck_raw: Optional[str] = None


def _pending_game_state(pending: PendingGame) -> dict:
    return {
        "game_id": pending.game_id,
        "join_code": pending.join_code,
        "host_display_name": pending.host_display_name,
        "host_deck_ready": bool(pending.host_deck),
        "joiner_deck_ready": False,
        "started": pending.game_id in active_games,
        "allow_spectators": pending.allow_spectators,
        "spectator_mode": pending.spectator_mode,
    }


def _parse_optional_deck(deck: list[str], deck_raw: Optional[str]) -> tuple[list[str], Optional[str]]:
    try:
        parsed = parse_deck(deck_raw) if deck_raw else deck
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=f"Deck parsing error: {exc}") from exc
    return parsed, deck_raw


# ── Endpoints ────────────────────────────────────────────────────────────

@router.post("/create")
async def create_lobby_game(
    request: CreateLobbyRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host creates a game and gets a join code.  Game is not started yet."""
    _prune_stale_lobbies()
    deck, deck_raw = _parse_optional_deck(request.deck, request.deck_raw)

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
        host_deck_raw=deck_raw,
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


@router.get("/{game_id}/state")
async def get_lobby_game(game_id: str) -> dict:
    """Return pending lobby readiness for the room screen."""
    _prune_stale_lobbies()
    pending = pending_games.get(game_id)
    if pending is None:
        if game_id in active_games:
            return {
                "game_id": game_id,
                "join_code": None,
                "host_display_name": None,
                "host_deck_ready": True,
                "joiner_deck_ready": True,
                "started": True,
            }
        raise HTTPException(status_code=404, detail="Pending game not found")
    return _pending_game_state(pending)


@router.put("/{game_id}/deck")
async def set_lobby_deck(
    game_id: str,
    request: SetLobbyDeckRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host locks or replaces the deck for a pending lobby."""
    _prune_stale_lobbies()
    pending = pending_games.get(game_id)
    if pending is None:
        raise HTTPException(status_code=404, detail="Pending game not found")
    if pending.host_user_id != user.id:
        raise HTTPException(status_code=403, detail="Only the host can set this deck")

    deck, deck_raw = _parse_optional_deck(request.deck, request.deck_raw)
    if not deck:
        raise HTTPException(status_code=400, detail="A deck must be provided")

    pending.host_deck = deck
    pending.host_deck_raw = deck_raw
    pending_games[game_id] = pending
    return _pending_game_state(pending)


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
    if not pending.host_deck:
        raise HTTPException(status_code=409, detail="Host deck is not locked yet")

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

    # Create the interactive game on the Rust engine (both players are human).
    try:
        runner = RustInteractiveGame(pending.host_deck, deck2)
    except Exception as exc:  # PyValueError from the Rust binding (illegal deck)
        raise HTTPException(status_code=400, detail=f"Engine construction failed: {exc}")

    active_games[game_id] = runner

    # Record the joiner's user_id so WebSocket slot validation works
    settings = manager.get_settings(game_id)
    settings.joiner_user_id = user.id

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
    _prune_stale_lobbies()
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
