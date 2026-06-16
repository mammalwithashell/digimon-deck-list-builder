"""Game lobby for network PvP matchmaking.

Players create rooms (with a shareable 5-digit numeric join code) or browse
public games. The room lifecycle is two-phase (add-room-match-pvp), mirroring
DCGO's host/guest flow:

    create(host) ─► waiting ─join(code)─► seated ─both decks─► ready
        ─start(host)─► started (RustHeadlessGame in ``active_games``)

Joining reserves a seat only — the joiner picks a deck *inside* the room and
the host explicitly starts the game once both decks are locked. Both clients
poll ``GET /lobby/{id}/state`` to drive the room UI and to learn their seat
when the game starts.

Games run on the Rust engine (``digimon_engine.RustHeadlessGame``); this
module imports nothing from ``engine_py_legacy`` (rule 22).
"""

from __future__ import annotations

import logging
import secrets
import string
from datetime import datetime, timedelta, timezone
from typing import Literal, Optional
from uuid import uuid4

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field, field_validator
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_engine import RustHeadlessGame, parse_deck

from server.db.auth import get_current_user
from server.db.database import get_db
from server.db.models import User
from server.routers.state import active_games
from server.routers.seed_utils import parse_optional_seed, seed_to_wire
from server.routers.ws_manager import GameSettings, manager

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/lobby", tags=["lobby"])

# ── In-memory lobby state ────────────────────────────────────────────────

_CODE_LENGTH = 5

FirstPlayerChoice = Literal["1", "random", "2"]


def _generate_join_code(length: int = _CODE_LENGTH) -> str:
    """5-digit numeric code (leading zeros preserved) — easy to read aloud."""
    return "".join(secrets.choice(string.digits) for _ in range(length))


class PendingGame(BaseModel):
    game_id: str
    join_code: str
    host_user_id: str
    host_display_name: str
    host_deck: list[str] = Field(default_factory=list)
    host_deck_raw: Optional[str] = None
    joiner_user_id: Optional[str] = None
    joiner_display_name: Optional[str] = None
    joiner_deck: list[str] = Field(default_factory=list)
    joiner_deck_raw: Optional[str] = None
    first_player: FirstPlayerChoice = "random"
    explicit_seed: Optional[int] = None
    effective_seed: Optional[int] = None
    started: bool = False
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
    """Remove pending lobbies that have exceeded the TTL.

    Started entries are kept as the seat map for ``/state`` until the TTL
    sweeps them too — but their WS connection tracking is left alone (the
    live game outlives the room entry; ``ws_games`` cleans up on game over).
    """
    cutoff = datetime.now(timezone.utc) - _LOBBY_TTL
    expired = [gid for gid, pg in pending_games.items() if pg.created_at < cutoff]
    for gid in expired:
        pg = pending_games.pop(gid)
        code_to_game.pop(pg.join_code, None)
        if not pg.started:
            manager.cleanup_game(gid)
    if expired:
        logger.info("Pruned %d stale lobbies", len(expired))


# ── Rust game construction ───────────────────────────────────────────────

def _seed_for_first_player(choice: FirstPlayerChoice) -> int:
    """Pick a game seed whose parity realizes the first-player choice.

    ``Game::new`` rotates the turn order by ``seed % player_count`` (see
    digimon-engine ``game/setup.rs``): an even seed puts player 1 first, an
    odd seed puts player 2 first. ``random`` leaves the parity to chance.
    """
    seed = secrets.randbits(62)
    if choice == "1":
        return seed - (seed % 2)
    if choice == "2":
        return seed | 1
    return seed


def seed_for_pvp_game(
    first_player: FirstPlayerChoice = "random",
    explicit_seed: Optional[int] = None,
) -> int:
    return explicit_seed if explicit_seed is not None else _seed_for_first_player(first_player)


def create_pvp_game(
    host_deck: list[str],
    joiner_deck: list[str],
    first_player: FirstPlayerChoice = "random",
    seed: Optional[int] = None,
) -> RustHeadlessGame:
    """Construct a two-human Rust game. Shared with matchmaking so quick
    match and room match go through the same construction path."""
    effective_seed = seed_for_pvp_game(first_player, seed)
    return RustHeadlessGame(host_deck, joiner_deck, False, False, False, effective_seed)


# ── Request / Response Schemas ───────────────────────────────────────────

class CreateLobbyRequest(BaseModel):
    deck: list[str] = Field(default_factory=list)
    deck_raw: Optional[str] = None
    is_public: bool = True
    allow_spectators: bool = True
    spectator_mode: str = Field("hidden", pattern="^(hidden|open)$")


class SetLobbyDeckRequest(BaseModel):
    deck: list[str] = Field(default_factory=list)
    deck_raw: Optional[str] = None


class SetFirstPlayerRequest(BaseModel):
    first_player: FirstPlayerChoice


class SetSeedRequest(BaseModel):
    seed: Optional[int | str] = None

    @field_validator("seed", mode="before")
    @classmethod
    def _validate_seed(cls, value):
        return parse_optional_seed(value)


def _seat_of(pending: PendingGame, user_id: str) -> Optional[int]:
    if pending.host_user_id == user_id:
        return 1
    if pending.joiner_user_id == user_id:
        return 2
    return None


def _pending_game_state(pending: PendingGame, user_id: Optional[str] = None) -> dict:
    visible_seed = pending.effective_seed if pending.started else pending.explicit_seed
    seed_mode = "explicit" if pending.explicit_seed is not None else "generated"
    return {
        "game_id": pending.game_id,
        "join_code": pending.join_code,
        "host_display_name": pending.host_display_name,
        "joiner_display_name": pending.joiner_display_name,
        "host_deck_ready": bool(pending.host_deck),
        "joiner_deck_ready": bool(pending.joiner_deck),
        "first_player": pending.first_player,
        "seed": seed_to_wire(visible_seed) if visible_seed is not None else None,
        "seed_mode": seed_mode,
        "first_player_seed_locked": pending.explicit_seed is not None and not pending.started,
        "started": pending.started,
        "your_seat": _seat_of(pending, user_id) if user_id else None,
        "allow_spectators": pending.allow_spectators,
        "spectator_mode": pending.spectator_mode,
    }


def _parse_optional_deck(deck: list[str], deck_raw: Optional[str]) -> tuple[list[str], Optional[str]]:
    try:
        parsed = parse_deck(deck_raw) if deck_raw else deck
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=f"Deck parsing error: {exc}") from exc
    return parsed, deck_raw


def _require_pending(game_id: str) -> PendingGame:
    pending = pending_games.get(game_id)
    if pending is None:
        raise HTTPException(status_code=404, detail="Pending game not found")
    return pending


# ── Endpoints ────────────────────────────────────────────────────────────

@router.post("/create")
async def create_lobby_game(
    request: CreateLobbyRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host creates a room and gets a join code.  Game is not started yet."""
    _prune_stale_lobbies()
    deck, deck_raw = _parse_optional_deck(request.deck, request.deck_raw)

    game_id = str(uuid4())
    join_code = _generate_join_code()

    # Ensure code uniqueness (1-in-100k collision per pending room)
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


@router.post("/join/{join_code}")
async def join_lobby_game(
    join_code: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Second player reserves the joiner seat. No deck required; the game is
    NOT created here — the joiner locks a deck in the room and the host
    starts the game explicitly."""
    join_code = join_code.strip()
    game_id = code_to_game.get(join_code)
    if game_id is None or game_id not in pending_games:
        raise HTTPException(status_code=404, detail="Game not found or already started")

    pending = pending_games[game_id]
    if pending.started:
        raise HTTPException(status_code=404, detail="Game not found or already started")

    if pending.host_user_id == user.id:
        raise HTTPException(status_code=400, detail="Cannot join your own game")

    if pending.joiner_user_id is not None and pending.joiner_user_id != user.id:
        raise HTTPException(status_code=409, detail="Room is full")

    # Idempotent re-join for the seated user
    if pending.joiner_user_id != user.id:
        pending.joiner_user_id = user.id
        pending.joiner_display_name = user.display_name or user.username
        logger.info(
            "Player %s seated in room %s (host: %s)",
            user.username, game_id, pending.host_display_name,
        )

    return {
        "game_id": game_id,
        "your_seat": 2,
        "state": _pending_game_state(pending, user.id),
    }


@router.get("/{game_id}/state")
async def get_lobby_game(
    game_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Return room readiness for the room screen. Authenticated so the
    response can carry the caller's seat; both clients poll this."""
    _prune_stale_lobbies()
    pending = pending_games.get(game_id)
    if pending is None:
        if game_id in active_games:
            # Room entry already pruned but the game lives on — report
            # started without seat info (the WS layer validates seats).
            return {
                "game_id": game_id,
                "join_code": None,
                "host_display_name": None,
                "joiner_display_name": None,
                "host_deck_ready": True,
                "joiner_deck_ready": True,
                "first_player": None,
                "started": True,
                "your_seat": None,
            }
        raise HTTPException(status_code=404, detail="Pending game not found")
    return _pending_game_state(pending, user.id)


@router.put("/{game_id}/deck")
async def set_lobby_deck(
    game_id: str,
    request: SetLobbyDeckRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host or seated joiner locks (or replaces) the deck for their seat."""
    _prune_stale_lobbies()
    pending = _require_pending(game_id)
    if pending.started:
        raise HTTPException(status_code=409, detail="Game already started")

    seat = _seat_of(pending, user.id)
    if seat is None:
        raise HTTPException(status_code=403, detail="Not a participant in this room")

    deck, deck_raw = _parse_optional_deck(request.deck, request.deck_raw)
    if not deck:
        raise HTTPException(status_code=400, detail="A deck must be provided")

    if seat == 1:
        pending.host_deck = deck
        pending.host_deck_raw = deck_raw
    else:
        pending.joiner_deck = deck
        pending.joiner_deck_raw = deck_raw
    return _pending_game_state(pending, user.id)


@router.put("/{game_id}/first-player")
async def set_first_player(
    game_id: str,
    request: SetFirstPlayerRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host picks who takes the first turn (DCGO: 1 / Random / 2)."""
    pending = _require_pending(game_id)
    if pending.host_user_id != user.id:
        raise HTTPException(status_code=403, detail="Only the host can set the first player")
    if pending.started:
        raise HTTPException(status_code=409, detail="Game already started")

    pending.first_player = request.first_player
    return _pending_game_state(pending, user.id)


@router.put("/{game_id}/seed")
async def set_room_seed(
    game_id: str,
    request: SetSeedRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host sets or clears the explicit room seed before start."""
    pending = _require_pending(game_id)
    if pending.host_user_id != user.id:
        raise HTTPException(status_code=403, detail="Only the host can set the game seed")
    if pending.started:
        raise HTTPException(status_code=409, detail="Game already started")

    pending.explicit_seed = request.seed if isinstance(request.seed, int) else None
    pending.effective_seed = None
    return _pending_game_state(pending, user.id)


@router.post("/{game_id}/leave")
async def leave_lobby_game(
    game_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Seated joiner vacates the seat (pre-start only)."""
    pending = _require_pending(game_id)
    if pending.started:
        raise HTTPException(status_code=409, detail="Game already started")
    if pending.joiner_user_id != user.id:
        raise HTTPException(status_code=403, detail="Only the seated joiner can leave")

    pending.joiner_user_id = None
    pending.joiner_display_name = None
    pending.joiner_deck = []
    pending.joiner_deck_raw = None
    return _pending_game_state(pending, user.id)


@router.post("/{game_id}/start")
async def start_lobby_game(
    game_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host starts the game (DCGO 'Finish Preparation'). Requires both seats
    occupied and both decks locked. Creates the Rust game exactly once."""
    pending = _require_pending(game_id)
    if pending.host_user_id != user.id:
        raise HTTPException(status_code=403, detail="Only the host can start the game")
    if pending.started:
        raise HTTPException(status_code=409, detail="Game already started")
    if pending.joiner_user_id is None:
        raise HTTPException(status_code=409, detail="No opponent has joined yet")
    if not pending.host_deck or not pending.joiner_deck:
        raise HTTPException(status_code=409, detail="Both decks must be locked")

    effective_seed = seed_for_pvp_game(pending.first_player, pending.explicit_seed)
    try:
        runner = create_pvp_game(
            pending.host_deck,
            pending.joiner_deck,
            pending.first_player,
            effective_seed,
        )
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=f"Game creation failed: {exc}") from exc

    active_games[game_id] = runner

    # Record the joiner's user_id so WebSocket slot validation works
    settings = manager.get_settings(game_id)
    settings.joiner_user_id = pending.joiner_user_id
    settings.seed = seed_to_wire(effective_seed)

    # Free the code; keep the (started) pending entry as the seat map for
    # `/state` polling until the TTL sweeps it.
    code_to_game.pop(pending.join_code, None)
    pending.started = True
    pending.effective_seed = effective_seed

    logger.info(
        "Room %s started: host=%s joiner=%s first_player=%s",
        game_id, pending.host_display_name, pending.joiner_display_name,
        pending.first_player,
    )

    return _pending_game_state(pending, user.id)


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
        if pg.is_public and not pg.started
    ]


@router.delete("/{game_id}")
async def cancel_lobby_game(
    game_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
) -> dict:
    """Host cancels a pending game (pre-start only)."""
    pending = _require_pending(game_id)
    if pending.host_user_id != user.id:
        raise HTTPException(status_code=403, detail="Only the host can cancel")
    if pending.started:
        raise HTTPException(status_code=409, detail="Game already started")

    code_to_game.pop(pending.join_code, None)
    del pending_games[game_id]
    manager.cleanup_game(game_id)

    return {"status": "cancelled"}
