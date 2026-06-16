"""WebSocket endpoint for matchmaking tickets.

Protocol (server → client only; no messages accepted from the client):

    {"type": "waiting",     "waited_seconds": float, "rating_window": float|None}
    {"type": "match_found", "game_id": str, "your_seat": int,
                            "opponent": {"user_id": str, "display_name": str}}
    {"type": "cancelled",   "reason": str}

Disconnecting while waiting auto-cancels the ticket so it doesn't sit in the
queue as a ghost. Matched tickets are retained regardless of WS state so the
REST `GET /matchmaking/queue/{id}` poller can still pick up the game.
"""
from __future__ import annotations

import asyncio
import logging
from typing import Any, Dict, Optional

from fastapi import APIRouter, WebSocket, WebSocketDisconnect
from jose import JWTError

from server.db.auth import decode_access_token
from server.routers import matchmaking as mm

logger = logging.getLogger(__name__)
router = APIRouter()

HEARTBEAT_INTERVAL_S = 5.0


def _authenticate_ws(token: str) -> Optional[Dict[str, Any]]:
    if not token:
        return None
    try:
        return decode_access_token(token)
    except JWTError:
        return None


def _match_found_payload(ticket: mm.QueueTicket) -> dict:
    opponent_name: Optional[str] = None
    opponent_id = ticket.matched_with_user_id
    if opponent_id is not None:
        for other in mm.tickets.values():
            if other.user_id == opponent_id and other.ticket_id != ticket.ticket_id:
                opponent_name = other.display_name
                break
    return {
        "type": "match_found",
        "game_id": ticket.matched_game_id,
        "your_seat": ticket.matched_seat,
        "opponent": {
            "user_id": opponent_id,
            "display_name": opponent_name,
        },
    }


def _waiting_payload(ticket: mm.QueueTicket) -> dict:
    from datetime import datetime, timezone
    now = datetime.now(timezone.utc)
    payload: dict = {
        "type": "waiting",
        "waited_seconds": (now - ticket.created_at).total_seconds(),
    }
    if ticket.queue_type == "ranked":
        payload["rating_window"] = mm.rating_window(ticket, now)
    return payload


@router.websocket("/ws/matchmaking/{ticket_id}")
async def matchmaking_websocket(websocket: WebSocket, ticket_id: str) -> None:
    token = websocket.query_params.get("token", "")
    payload = _authenticate_ws(token)
    if payload is None:
        await websocket.close(code=4001, reason="Invalid or missing auth token")
        return

    user_id = payload["sub"]
    ticket = mm.tickets.get(ticket_id)
    if ticket is None:
        await websocket.close(code=4004, reason="Ticket not found")
        return
    if ticket.user_id != user_id:
        await websocket.close(code=4003, reason="Not your ticket")
        return

    await websocket.accept()

    # Fast path: ticket already matched by the time the socket opens.
    if ticket.status == "matched":
        await websocket.send_json(_match_found_payload(ticket))
        await websocket.close(code=1000)
        return

    # Register for match notifications.
    event = mm.get_or_create_listener(ticket_id)
    await websocket.send_json(_waiting_payload(ticket))

    # Run a background task that reads from the socket so we notice client
    # disconnects promptly (within one awaited step) instead of waiting for
    # the heartbeat timeout. We don't expect any messages from the client —
    # receive just raises WebSocketDisconnect when the socket closes.
    disconnect_task = asyncio.create_task(_wait_for_disconnect(websocket))
    event_task: Optional[asyncio.Task] = None
    disconnected = False
    try:
        while True:
            if event_task is None or event_task.done():
                event_task = asyncio.create_task(event.wait())
            done, _ = await asyncio.wait(
                {event_task, disconnect_task},
                timeout=HEARTBEAT_INTERVAL_S,
                return_when=asyncio.FIRST_COMPLETED,
            )
            if disconnect_task in done:
                disconnected = True
                break
            if not done:
                # Heartbeat tick — keep-alive + updated rating window
                current = mm.tickets.get(ticket_id)
                if current is None:
                    await websocket.send_json({
                        "type": "cancelled", "reason": "ticket_removed",
                    })
                    break
                await websocket.send_json(_waiting_payload(current))
                continue

            # Event fired — ticket matched, cancelled, or pruned.
            current = mm.tickets.get(ticket_id)
            if current is None:
                await websocket.send_json({
                    "type": "cancelled", "reason": "ticket_removed",
                })
                break
            if current.status == "matched":
                await websocket.send_json(_match_found_payload(current))
                break
            # Spurious wake. Reset and keep waiting.
            event.clear()
            event_task = None
    except WebSocketDisconnect:
        disconnected = True
    finally:
        disconnect_task.cancel()
        if event_task is not None:
            event_task.cancel()
        # If the client dropped while we were still waiting, drop the
        # ticket — no sense keeping a ghost in the queue.
        current = mm.tickets.get(ticket_id)
        if disconnected and current is not None and current.status == "waiting":
            mm.cancel_waiting_ticket(ticket_id)
        # The listener is shared across reconnects; drop it only if this
        # was the terminal state.
        if current is None or current.status != "waiting":
            mm.drop_listener(ticket_id)

    if not disconnected:
        try:
            await websocket.close(code=1000)
        except RuntimeError:
            pass


async def _wait_for_disconnect(ws: WebSocket) -> None:
    """Sits on ws.receive() and waits for the client to go away. We don't
    act on any received message — the matchmaking protocol is server-push
    only — but receive() is how asyncio learns about the disconnect."""
    try:
        while True:
            await ws.receive()
    except WebSocketDisconnect:
        return
