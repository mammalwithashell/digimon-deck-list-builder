"""Matchmaking queue — casual (tier-filtered) + ranked (rating-window) tickets.

State lives in-memory in the API process, mirroring the existing lobby
`pending_games` pattern. Matched pairs are handed off to the existing
`/lobby/join/{code}` pipeline by synthesizing a join code server-side, so no
game-creation logic is duplicated.

Public contracts consumed by the matcher:
    - `Deck.meta_tier`   → populated by `digimon_gym.classifier.deck_tagger`
    - `Deck.game_mode`   → format filter (standard / no_restriction / ...)
    - `User.rating`      → opaque scalar, updated by the ranked-rating spec
"""
from __future__ import annotations

import asyncio
import logging
import math
import secrets
import string
from datetime import datetime, timedelta, timezone
from typing import Literal, Optional
from uuid import uuid4

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel, Field
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.auth import get_current_user
from digimon_gym.db.database import get_db
from digimon_gym.db.models import Deck, User
from digimon_gym.engine.runners.interactive_game import InteractiveGame  # noqa: F401
from digimon_gym.routers.lobby import (
    PendingGame,
    _generate_join_code,
    code_to_game,
    pending_games,
)
from digimon_gym.routers.ws_manager import GameSettings, manager

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/matchmaking", tags=["matchmaking"])


# ── Types ───────────────────────────────────────────────────────────────

QueueType = Literal["casual", "ranked"]
TierFilter = Literal["same", "any", "meta_only", "jank_only"]
TicketStatus = Literal["waiting", "matched", "cancelled"]

# Rating-window tuning (plan §Matcher "Ranked"):
RATING_WINDOW_INITIAL = 50.0
RATING_WINDOW_STEP = 50.0          # +50 per RATING_WINDOW_INTERVAL_S
RATING_WINDOW_INTERVAL_S = 5.0
RATING_WINDOW_CAP = 400.0

# Sweep cadence for ranked re-evaluation (when window widens over time)
SWEEP_INTERVAL_S = 2.0
# How long to keep a matched ticket around so the other side can poll it up
MATCHED_RETENTION_S = 120.0
# How long a waiting ticket is allowed to sit before being pruned
WAITING_TTL_S = 600.0


class QueueTicket(BaseModel):
    ticket_id: str
    user_id: str
    display_name: str
    queue_type: QueueType
    deck: list[str]
    deck_raw: Optional[str] = None
    game_mode: str                        # acts as the "format" filter
    self_tier: Optional[str] = None       # classifier output, snapshotted at queue time
    opponent_tier_filter: TierFilter = "any"
    rating: Optional[float] = None        # ranked only
    created_at: datetime
    status: TicketStatus = "waiting"
    matched_with_user_id: Optional[str] = None
    matched_join_code: Optional[str] = None
    matched_game_id: Optional[str] = None
    matched_at: Optional[datetime] = None


# ── In-memory registry ──────────────────────────────────────────────────

tickets: dict[str, QueueTicket] = {}
user_to_ticket: dict[str, str] = {}
# ticket_id → asyncio.Event. Signalled when the ticket transitions to
# "matched" (or is cancelled); the WS handler awaits this event so it can
# push `match_found` without polling.
_match_events: dict[str, "asyncio.Event"] = {}


def reset_state() -> None:
    """Test-hook: wipe in-memory queue state."""
    tickets.clear()
    user_to_ticket.clear()
    _match_events.clear()


def get_or_create_listener(ticket_id: str) -> "asyncio.Event":
    """Return the asyncio.Event that will fire when `ticket_id` is matched
    or removed. Called by the WS handler on connect."""
    ev = _match_events.get(ticket_id)
    if ev is None:
        ev = asyncio.Event()
        _match_events[ticket_id] = ev
    return ev


def _fire_listener(ticket_id: str) -> None:
    ev = _match_events.get(ticket_id)
    if ev is not None:
        ev.set()


def drop_listener(ticket_id: str) -> None:
    _match_events.pop(ticket_id, None)


# ── Pure matcher ────────────────────────────────────────────────────────

def rating_window(ticket: QueueTicket, now: datetime) -> float:
    elapsed = max(0.0, (now - ticket.created_at).total_seconds())
    steps = math.floor(elapsed / RATING_WINDOW_INTERVAL_S)
    return min(RATING_WINDOW_INITIAL + RATING_WINDOW_STEP * steps, RATING_WINDOW_CAP)


def _casual_compatible(a: QueueTicket, b: QueueTicket) -> bool:
    """Symmetric tier-filter compatibility."""
    return _accepts(a, b) and _accepts(b, a)


def _accepts(self_ticket: QueueTicket, opponent: QueueTicket) -> bool:
    """Does `self_ticket.opponent_tier_filter` accept `opponent.self_tier`?"""
    f = self_ticket.opponent_tier_filter
    tier = opponent.self_tier
    if f == "any":
        return True
    if f == "meta_only":
        return tier == "meta"
    if f == "jank_only":
        return tier == "jank"
    if f == "same":
        return tier == self_ticket.self_tier
    return False


def _ranked_compatible(a: QueueTicket, b: QueueTicket, now: datetime) -> bool:
    if a.rating is None or b.rating is None:
        return False
    gap = abs(a.rating - b.rating)
    return gap <= rating_window(a, now) and gap <= rating_window(b, now)


def find_match(
    incoming: QueueTicket,
    pool: list[QueueTicket],
    *,
    now: datetime,
) -> Optional[QueueTicket]:
    """FIFO: return the oldest compatible `waiting` ticket, or None."""
    candidates = [
        t for t in pool
        if t.ticket_id != incoming.ticket_id
        and t.status == "waiting"
        and t.user_id != incoming.user_id
        and t.queue_type == incoming.queue_type
        and t.game_mode == incoming.game_mode
    ]
    if incoming.queue_type == "casual":
        candidates = [t for t in candidates if _casual_compatible(incoming, t)]
    else:
        candidates = [t for t in candidates if _ranked_compatible(incoming, t, now)]

    candidates.sort(key=lambda t: t.created_at)
    return candidates[0] if candidates else None


# ── Request / response schemas ──────────────────────────────────────────

class QueueRequest(BaseModel):
    queue_type: QueueType
    deck_id: str
    opponent_tier_filter: TierFilter = "any"


class TicketInfoResponse(BaseModel):
    ticket_id: str
    status: TicketStatus
    queue_type: QueueType
    waited_seconds: float
    rating_window: Optional[float] = None
    join_code: Optional[str] = None
    game_id: Optional[str] = None


# ── Match handoff ───────────────────────────────────────────────────────

def _create_pending_game_from_match(
    host_ticket: QueueTicket,
    joiner_ticket: QueueTicket,
) -> tuple[str, str]:
    """Synthesize a PendingGame + join code. Mirrors lobby.create_lobby_game
    so the joiner can use the existing `/lobby/join/{code}` path verbatim."""
    game_id = str(uuid4())
    join_code = _generate_join_code()
    while join_code in code_to_game:
        join_code = _generate_join_code()

    pending = PendingGame(
        game_id=game_id,
        join_code=join_code,
        host_user_id=host_ticket.user_id,
        host_display_name=host_ticket.display_name,
        host_deck=host_ticket.deck,
        host_deck_raw=host_ticket.deck_raw,
        created_at=datetime.now(timezone.utc),
        is_public=False,             # hidden from the public lobby browser
        allow_spectators=False,
        spectator_mode="hidden",
    )
    pending_games[game_id] = pending
    code_to_game[join_code] = game_id
    manager.set_settings(game_id, GameSettings(
        allow_spectators=False,
        spectator_mode="hidden",
        host_user_id=host_ticket.user_id,
    ))
    logger.info(
        "matchmaking: paired %s vs %s as game_id=%s code=%s",
        host_ticket.user_id, joiner_ticket.user_id, game_id, join_code,
    )
    return (game_id, join_code)


def _promote_to_matched(
    incoming: QueueTicket,
    opponent: QueueTicket,
) -> tuple[str, str]:
    """Create the lobby handoff and mark both tickets as matched.
    The older ticket is host (it's been waiting longer); the incoming ticket
    is joiner."""
    host, joiner = (opponent, incoming) if opponent.created_at <= incoming.created_at else (incoming, opponent)
    game_id, join_code = _create_pending_game_from_match(host, joiner)
    now = datetime.now(timezone.utc)
    for side, other in ((host, joiner), (joiner, host)):
        side.status = "matched"
        side.matched_with_user_id = other.user_id
        side.matched_join_code = join_code
        side.matched_game_id = game_id
        side.matched_at = now
        _fire_listener(side.ticket_id)
    return (game_id, join_code)


def cancel_waiting_ticket(ticket_id: str) -> bool:
    """Remove a waiting ticket from the registry. No-op (returns False) if
    the ticket is matched or unknown. Used by the WS handler on client
    disconnect to prevent ghost entries."""
    t = tickets.get(ticket_id)
    if t is None or t.status != "waiting":
        return False
    tickets.pop(ticket_id, None)
    if user_to_ticket.get(t.user_id) == ticket_id:
        user_to_ticket.pop(t.user_id, None)
    _fire_listener(ticket_id)
    return True


# ── REST endpoints ──────────────────────────────────────────────────────

@router.post("/queue")
async def enqueue(
    request: QueueRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    """Submit a matchmaking ticket. Immediately returns:
        - 200 `{status: "matched", ticket_id, join_code, game_id}` if a
          compatible opponent was already waiting;
        - 201 `{status: "waiting", ticket_id}` otherwise.
    """
    _prune_stale_tickets()

    if user.id in user_to_ticket:
        raise HTTPException(status.HTTP_409_CONFLICT, "User already has an active ticket")

    result = await db.execute(select(Deck).where(Deck.id == request.deck_id))
    deck = result.scalar_one_or_none()
    if deck is None or deck.owner_id != user.id:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "Deck not found")

    import json
    card_ids = json.loads(deck.main_deck) + json.loads(deck.egg_deck or "[]")
    if not card_ids:
        raise HTTPException(status.HTTP_400_BAD_REQUEST, "Deck is empty")

    rating = None
    if request.queue_type == "ranked":
        rating = float(getattr(user, "rating", 1500.0) or 1500.0)

    ticket = QueueTicket(
        ticket_id=str(uuid4()),
        user_id=user.id,
        display_name=user.display_name or user.username,
        queue_type=request.queue_type,
        deck=card_ids,
        game_mode=deck.game_mode,
        self_tier=deck.meta_tier,
        opponent_tier_filter=request.opponent_tier_filter,
        rating=rating,
        created_at=datetime.now(timezone.utc),
    )

    opponent = find_match(ticket, list(tickets.values()), now=ticket.created_at)
    if opponent is not None:
        # Register the incoming ticket first so state is consistent, then
        # promote both.
        tickets[ticket.ticket_id] = ticket
        user_to_ticket[ticket.user_id] = ticket.ticket_id
        game_id, join_code = _promote_to_matched(ticket, opponent)
        return {
            "status": "matched",
            "ticket_id": ticket.ticket_id,
            "opponent_ticket_id": opponent.ticket_id,
            "game_id": game_id,
            "join_code": join_code,
        }

    tickets[ticket.ticket_id] = ticket
    user_to_ticket[user.id] = ticket.ticket_id
    return _queued_response(ticket)


def _queued_response(ticket: QueueTicket):
    from fastapi.responses import JSONResponse
    return JSONResponse(
        status_code=201,
        content={"status": "waiting", "ticket_id": ticket.ticket_id},
    )


@router.get("/queue/{ticket_id}", response_model=TicketInfoResponse)
async def get_ticket(
    ticket_id: str,
    user: User = Depends(get_current_user),
):
    ticket = tickets.get(ticket_id)
    if ticket is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "Ticket not found")
    if ticket.user_id != user.id:
        raise HTTPException(status.HTTP_403_FORBIDDEN, "Not your ticket")

    now = datetime.now(timezone.utc)
    waited = (now - ticket.created_at).total_seconds()
    win = rating_window(ticket, now) if ticket.queue_type == "ranked" else None
    return TicketInfoResponse(
        ticket_id=ticket.ticket_id,
        status=ticket.status,
        queue_type=ticket.queue_type,
        waited_seconds=waited,
        rating_window=win,
        join_code=ticket.matched_join_code,
        game_id=ticket.matched_game_id,
    )


@router.delete("/queue/{ticket_id}")
async def cancel_ticket(
    ticket_id: str,
    user: User = Depends(get_current_user),
):
    ticket = tickets.get(ticket_id)
    if ticket is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "Ticket not found")
    if ticket.user_id != user.id:
        raise HTTPException(status.HTTP_403_FORBIDDEN, "Not your ticket")
    if ticket.status == "matched":
        raise HTTPException(status.HTTP_409_CONFLICT, "Ticket already matched")

    tickets.pop(ticket_id, None)
    user_to_ticket.pop(user.id, None)
    _fire_listener(ticket_id)
    return {"status": "cancelled"}


# ── Housekeeping ────────────────────────────────────────────────────────

def _prune_stale_tickets() -> None:
    """Drop matched tickets past their grace period and waiting tickets
    past their TTL. Called opportunistically on queue POST and by the
    background sweep."""
    now = datetime.now(timezone.utc)
    stale_ids: list[str] = []
    for tid, t in tickets.items():
        if t.status == "matched" and t.matched_at is not None:
            if (now - t.matched_at).total_seconds() > MATCHED_RETENTION_S:
                stale_ids.append(tid)
        elif t.status == "waiting":
            if (now - t.created_at).total_seconds() > WAITING_TTL_S:
                stale_ids.append(tid)
    for tid in stale_ids:
        t = tickets.pop(tid, None)
        if t is not None and user_to_ticket.get(t.user_id) == tid:
            user_to_ticket.pop(t.user_id, None)


async def sweep_once() -> None:
    """One pass over ranked waiting tickets; re-runs find_match so that
    rating-window expansion produces new pairings over time."""
    _prune_stale_tickets()
    now = datetime.now(timezone.utc)
    ranked_waiting = sorted(
        (t for t in tickets.values()
         if t.queue_type == "ranked" and t.status == "waiting"),
        key=lambda t: t.created_at,
    )
    for t in ranked_waiting:
        if t.status != "waiting":
            continue
        opponent = find_match(t, list(tickets.values()), now=now)
        if opponent is not None:
            _promote_to_matched(t, opponent)


_sweep_task: Optional[asyncio.Task] = None


async def _sweep_loop() -> None:
    while True:
        try:
            await asyncio.sleep(SWEEP_INTERVAL_S)
            await sweep_once()
        except asyncio.CancelledError:
            raise
        except Exception:  # pragma: no cover - keep the loop alive
            logger.exception("matchmaking sweep error")


async def start_sweep() -> None:
    global _sweep_task
    if _sweep_task is None or _sweep_task.done():
        _sweep_task = asyncio.create_task(_sweep_loop())


async def stop_sweep() -> None:
    global _sweep_task
    if _sweep_task is not None:
        _sweep_task.cancel()
        try:
            await _sweep_task
        except (asyncio.CancelledError, Exception):
            pass
        _sweep_task = None
