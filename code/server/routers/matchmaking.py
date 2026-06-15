"""Matchmaking queue — four queue types, alpha gated.

Queues:
    - ``jank``   : both decks must be classified ``jank``
    - ``casual`` : any-vs-any, no tier constraint, FIFO
    - ``sweat``  : both decks must be ``meta`` or ``rogue`` (tournament-shape)
    - ``ranked`` : rating-window widening over time; gated behind the
                   ``MATCHMAKING_RANKED_ENABLED`` env var (off for alpha)

State lives in-memory in the API process, mirroring the existing lobby
`pending_games` pattern. Matched pairs are handed off to the existing
`/lobby/join/{code}` pipeline by synthesizing a join code server-side, so no
game-creation logic is duplicated.

Public contracts consumed by the matcher:
    - `Deck.meta_tier`   → populated by `server.classifier.deck_tagger`
    - `Deck.game_mode`   → format filter (standard / no_restriction / ...)
    - `User.rating`      → opaque scalar, updated by the ranked-rating spec
"""
from __future__ import annotations

import asyncio
import logging
import math
import os
import secrets
import string
from datetime import datetime, timedelta, timezone
from typing import Literal, Optional
from uuid import uuid4

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel, Field
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from server.db.auth import get_current_user
from server.db.database import get_db
from server.db.models import Deck, User
from digimon_engine import validate_deck
from server.routers.lobby import (
    PendingGame,
    _generate_join_code,
    code_to_game,
    pending_games,
)
from server.routers.ws_manager import GameSettings, manager

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/matchmaking", tags=["matchmaking"])


# ── Types ───────────────────────────────────────────────────────────────

QueueType = Literal["jank", "casual", "sweat", "ranked"]
TicketStatus = Literal["waiting", "matched", "cancelled"]

# Alpha public queues — ranked is gated behind MATCHMAKING_RANKED_ENABLED.
# Typed against QueueType so adding a new queue value surfaces a type error
# here if it isn't classified as alpha vs. gated.
ALPHA_QUEUES: tuple[QueueType, ...] = ("jank", "casual", "sweat")
SWEAT_TIERS: frozenset[str] = frozenset({"meta", "rogue"})


def ranked_enabled() -> bool:
    """Runtime gate for the ranked queue. Read on every call so tests and
    ops can flip the flag without process restarts."""
    return os.getenv("MATCHMAKING_RANKED_ENABLED", "0") == "1"

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
    """FIFO: return the oldest compatible `waiting` ticket, or None.

    Queue semantics:
    - jank / casual / sweat: queue_type IS the tier filter. Tier eligibility
      is enforced at enqueue (422 rejection), so any ticket already in the
      pool with matching queue_type + game_mode is compatible.
    - ranked: rating-window compatibility, widening over time.
    """
    candidates = [
        t for t in pool
        if t.ticket_id != incoming.ticket_id
        and t.status == "waiting"
        and t.user_id != incoming.user_id
        and t.queue_type == incoming.queue_type
        and t.game_mode == incoming.game_mode
    ]
    if incoming.queue_type == "ranked":
        candidates = [t for t in candidates if _ranked_compatible(incoming, t, now)]

    candidates.sort(key=lambda t: t.created_at)
    return candidates[0] if candidates else None


# ── Request / response schemas ──────────────────────────────────────────

class QueueRequest(BaseModel):
    queue_type: QueueType
    # Either deck_id (resolves a server-side Deck row) or the inline
    # triple below. Inline is the guest path (no server-side Deck row);
    # deck_id is the accounts path. Inline decks cannot queue for jank or
    # sweat because those require a classifier-assigned tier that only
    # lives on server-side Deck rows.
    deck_id: Optional[str] = None
    main_deck: Optional[list[str]] = None
    egg_deck: Optional[list[str]] = None
    game_mode: Optional[str] = None


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

@router.get("/config")
async def get_matchmaking_config() -> dict:
    """Public queue-discovery endpoint. Lets the UI hide queues that are
    gated off without trial-and-error 403s. No auth required (no secrets
    exposed — just which queues are currently open)."""
    enabled = ranked_enabled()
    queues = list(ALPHA_QUEUES)
    if enabled:
        queues.append("ranked")
    return {"ranked_enabled": enabled, "queues": queues}


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

    # 409 (already queued) takes precedence over the 403 ranked-gate so a
    # user with an active ticket gets the same duplicate-ticket error
    # regardless of which queue they tried to double-enqueue into.
    if user.id in user_to_ticket:
        raise HTTPException(status.HTTP_409_CONFLICT, "User already has an active ticket")

    if request.queue_type == "ranked" and not ranked_enabled():
        raise HTTPException(
            status.HTTP_403_FORBIDDEN,
            "Ranked matchmaking is not available",
        )

    # Resolve the deck. Two paths:
    # - Inline payload (guest): request carries main_deck + egg_deck +
    #   game_mode, no server-side Deck row exists. self_tier stays None.
    # - deck_id (accounts): look up the server-side Deck row, snapshot
    #   its meta_tier as self_tier for jank/sweat eligibility.
    self_tier: Optional[str] = None
    if request.main_deck is not None:
        if request.game_mode is None:
            raise HTTPException(
                status.HTTP_400_BAD_REQUEST,
                "Inline deck requires game_mode",
            )
        card_ids = list(request.main_deck) + list(request.egg_deck or [])
        if not card_ids:
            raise HTTPException(status.HTTP_400_BAD_REQUEST, "Deck is empty")
        game_mode = request.game_mode
    elif request.deck_id is not None:
        result = await db.execute(select(Deck).where(Deck.id == request.deck_id))
        deck = result.scalar_one_or_none()
        if deck is None or deck.owner_id != user.id:
            raise HTTPException(status.HTTP_404_NOT_FOUND, "Deck not found")
        import json
        card_ids = json.loads(deck.main_deck) + json.loads(deck.egg_deck or "[]")
        if not card_ids:
            raise HTTPException(status.HTTP_400_BAD_REQUEST, "Deck is empty")
        game_mode = deck.game_mode
        self_tier = deck.meta_tier
    else:
        raise HTTPException(
            status.HTTP_400_BAD_REQUEST,
            "Request must include either deck_id or inline deck (main_deck + game_mode)",
        )

    # Defense-in-depth: re-validate against the standard ban list at enqueue
    # time. Deck save already enforces this (decks.py), but a deck saved
    # before a card was banned — or tampered with via admin / direct DB —
    # would otherwise bypass the format. no_restriction opts out explicitly.
    # Applies equally to the inline-deck path so guests can't queue a
    # banned card either.
    if game_mode != "no_restriction":
        # Rust validate_deck uses the official ENG restricted list internally;
        # Python's matched it as the default. Single-arg call covers both.
        ban_result = validate_deck(card_ids)
        if not ban_result.is_valid:
            raise HTTPException(
                status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
                detail={
                    "message": "Deck violates format restrictions",
                    "errors": ban_result.errors,
                    "game_mode": game_mode,
                },
            )

    # Queue-specific tier eligibility. Jank/sweat require a specific
    # classifier tier; casual + ranked have no tier gate. Inline-deck
    # (guest) tickets have `self_tier=None` and are therefore ineligible
    # for jank/sweat — guests can queue for casual or (if enabled) ranked.
    if request.queue_type == "jank" and self_tier != "jank":
        raise HTTPException(
            status.HTTP_422_UNPROCESSABLE_ENTITY,
            f"Deck tier {self_tier!r} is not eligible for jank queue",
        )
    if request.queue_type == "sweat" and self_tier not in SWEAT_TIERS:
        raise HTTPException(
            status.HTTP_422_UNPROCESSABLE_ENTITY,
            f"Deck tier {self_tier!r} is not eligible for sweat queue",
        )

    rating = None
    if request.queue_type == "ranked":
        rating = float(getattr(user, "rating", 1500.0) or 1500.0)

    ticket = QueueTicket(
        ticket_id=str(uuid4()),
        user_id=user.id,
        display_name=user.display_name or user.username,
        queue_type=request.queue_type,
        deck=card_ids,
        game_mode=game_mode,
        self_tier=self_tier,
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
