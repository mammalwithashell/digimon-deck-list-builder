"""WebSocket endpoint for real-time multiplayer games and spectating."""

from __future__ import annotations

import logging
from typing import Any, Dict, Optional

from fastapi import APIRouter, WebSocket, WebSocketDisconnect
from jose import JWTError

from digimon_gym.db.auth import decode_access_token
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.state_filter import filter_state_for_player
from digimon_gym.routers.state import active_games
from digimon_gym.routers.ws_manager import manager

logger = logging.getLogger(__name__)
router = APIRouter()


def _authenticate_ws(token: str) -> Optional[Dict[str, Any]]:
    """Validate a JWT token and return the payload, or None on failure."""
    if not token:
        return None
    try:
        return decode_access_token(token)
    except JWTError:
        return None


@router.websocket("/ws/games/{game_id}")
async def game_websocket(websocket: WebSocket, game_id: str) -> None:
    """WebSocket endpoint for live game participation or spectating.

    Query params:
        token  – JWT access token (required)
        role   – "player" or "spectator" (default: "player")
    """
    token = websocket.query_params.get("token", "")
    role = websocket.query_params.get("role", "player")

    # Authenticate
    payload = _authenticate_ws(token)
    if payload is None:
        await websocket.close(code=4001, reason="Invalid or missing auth token")
        return

    user_id: str = payload["sub"]
    username: str = payload.get("username", "Unknown")

    # Validate game exists
    runner = active_games.get(game_id)
    if runner is None:
        await websocket.close(code=4004, reason="Game not found")
        return

    if not isinstance(runner, InteractiveGame):
        await websocket.close(code=4004, reason="Game does not support WebSocket")
        return

    await websocket.accept()

    # Determine player assignment
    player_id: Optional[int] = None

    if role == "player":
        # Assign player_id based on lobby metadata stored in manager settings
        settings = manager.get_settings(game_id)
        if settings.host_user_id == user_id:
            player_id = 1
        elif manager.player_count(game_id) < 2:
            # Second player to connect
            player_id = 2 if manager.player_id_for(game_id, websocket) is None else None
            # Check if player 1 slot is actually taken
            if player_id == 2:
                conn = manager._games.get(game_id)
                if conn and 1 not in conn.players:
                    player_id = 1
        else:
            # Both slots full — check if reconnecting
            conn = manager._games.get(game_id)
            if conn:
                # Allow reconnection if user was previously connected
                # (In a production system you'd track user_id → player_id mapping)
                pass

        if player_id is None:
            # Game full, downgrade to spectator
            role = "spectator"

    if role == "spectator":
        await manager.connect_spectator(game_id, websocket)
        # Send current state
        full_state = runner.game.to_ui_json()
        from digimon_gym.engine.state_filter import filter_state_for_spectator

        spec_state = filter_state_for_spectator(
            full_state, manager.get_settings(game_id).spectator_mode
        )
        await websocket.send_json({
            "type": "state_update",
            "state": spec_state,
            "current_player_id": runner.game.current_player_id,
            "is_game_over": runner.is_game_over,
        })
        await _spectator_loop(websocket, game_id)
        return

    # Player connection
    await manager.connect_player(game_id, player_id, websocket)

    # Notify everyone
    await manager.broadcast_event(game_id, {
        "type": "player_joined",
        "player_id": player_id,
        "display_name": username,
    })

    # Send current state to the joining player
    full_state = runner.game.to_ui_json()
    filtered = filter_state_for_player(full_state, player_id)
    mask = runner.get_action_mask().tolist()
    current_pid = runner.game.current_player_id
    await websocket.send_json({
        "type": "state_update",
        "state": filtered,
        "action_mask": mask if current_pid == player_id else [],
        "action_descriptions": (
            runner.game.describe_actions(current_pid) if current_pid == player_id else {}
        ),
        "current_player_id": current_pid,
        "is_game_over": runner.is_game_over,
        "your_player_id": player_id,
    })

    # Send spectator count
    await manager.broadcast_event(game_id, {
        "type": "spectator_count",
        "count": manager.spectator_count(game_id),
    })

    # Main message loop
    try:
        while True:
            data = await websocket.receive_json()
            msg_type = data.get("type")

            if msg_type == "ping":
                await websocket.send_json({"type": "pong"})
                continue

            if msg_type == "action":
                await _handle_action(websocket, game_id, player_id, data, runner)
                continue

            await websocket.send_json({
                "type": "error",
                "message": f"Unknown message type: {msg_type}",
            })

    except WebSocketDisconnect:
        disconnected_pid = await manager.disconnect(game_id, websocket)
        if disconnected_pid is not None:
            await manager.broadcast_event(game_id, {
                "type": "player_disconnected",
                "player_id": disconnected_pid,
            })
    except Exception:
        logger.exception("WebSocket error in game %s", game_id)
        await manager.disconnect(game_id, websocket)


async def _handle_action(
    ws: WebSocket,
    game_id: str,
    player_id: int,
    data: Dict[str, Any],
    runner: InteractiveGame,
) -> None:
    """Process an action message from a player."""
    action_id = data.get("action_id")
    if action_id is None:
        await ws.send_json({"type": "error", "message": "Missing action_id"})
        return

    # Validate turn
    if runner.game.current_player_id != player_id:
        await ws.send_json({"type": "error", "message": "Not your turn"})
        return

    # Validate action is legal
    mask = runner.get_action_mask()
    if action_id < 0 or action_id >= len(mask) or mask[action_id] < 0.5:
        await ws.send_json({"type": "error", "message": "Invalid action"})
        return

    # Execute action
    runner.step(action_id)

    # Collect logs
    logs = runner.get_last_log()
    runner.clear_log()

    # Broadcast updated state to all
    await manager.broadcast_state(game_id, runner, logs=logs)

    # If game is over, send game_over event
    if runner.is_game_over:
        await manager.broadcast_event(game_id, {
            "type": "game_over",
            "winner_id": runner.game.winner.player_id if runner.game.winner else None,
        })


async def _spectator_loop(ws: WebSocket, game_id: str) -> None:
    """Listen for spectator messages (only ping is valid)."""
    try:
        while True:
            data = await ws.receive_json()
            msg_type = data.get("type")
            if msg_type == "ping":
                await ws.send_json({"type": "pong"})
            else:
                await ws.send_json({
                    "type": "error",
                    "message": "Spectators cannot send actions",
                })
    except WebSocketDisconnect:
        await manager.disconnect(game_id, ws)
    except Exception:
        logger.debug("Spectator WebSocket error", exc_info=True)
        await manager.disconnect(game_id, ws)
