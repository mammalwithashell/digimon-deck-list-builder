"""WebSocket connection manager for multiplayer games and spectating."""

from __future__ import annotations

import asyncio
import logging
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from starlette.websockets import WebSocket, WebSocketState

from engine_py_legacy.engine.runners.interactive_game import InteractiveGame
from server.state_filter import (
    filter_state_for_player,
    filter_state_for_spectator,
)

logger = logging.getLogger(__name__)


@dataclass
class GameSettings:
    """Per-game lobby settings that affect spectating and gameplay."""

    allow_spectators: bool = True
    spectator_mode: str = "hidden"  # "hidden" or "open"
    host_user_id: Optional[str] = None
    joiner_user_id: Optional[str] = None  # Set when player 2 joins via lobby


@dataclass
class GameConnections:
    """All WebSocket connections for a single game."""

    players: Dict[int, WebSocket] = field(default_factory=dict)
    spectators: List[WebSocket] = field(default_factory=list)
    # Map WebSocket → player_id for reverse lookups
    ws_to_player: Dict[int, int] = field(default_factory=dict)
    # Map user_id → player_id for reconnection and slot validation
    user_to_player: Dict[str, int] = field(default_factory=dict)
    settings: GameSettings = field(default_factory=GameSettings)


class ConnectionManager:
    """Tracks WebSocket connections per game and broadcasts state updates."""

    def __init__(self) -> None:
        self._games: Dict[str, GameConnections] = {}

    def _ensure_game(self, game_id: str) -> GameConnections:
        if game_id not in self._games:
            self._games[game_id] = GameConnections()
        return self._games[game_id]

    def set_settings(self, game_id: str, settings: GameSettings) -> None:
        conn = self._ensure_game(game_id)
        conn.settings = settings

    def get_settings(self, game_id: str) -> GameSettings:
        return self._ensure_game(game_id).settings

    # ------------------------------------------------------------------
    # Connection lifecycle
    # ------------------------------------------------------------------

    async def connect_player(
        self, game_id: str, player_id: int, ws: WebSocket, user_id: str | None = None
    ) -> None:
        conn = self._ensure_game(game_id)
        old_ws = conn.players.get(player_id)
        if old_ws is not None and old_ws is not ws:
            # Reconnect: close old socket silently
            try:
                await old_ws.close(code=4001, reason="Reconnected from another client")
            except Exception:
                pass
        conn.players[player_id] = ws
        conn.ws_to_player[id(ws)] = player_id
        if user_id is not None:
            conn.user_to_player[user_id] = player_id
        logger.info("Player %d connected to game %s", player_id, game_id)

    async def connect_spectator(self, game_id: str, ws: WebSocket) -> None:
        conn = self._ensure_game(game_id)
        if not conn.settings.allow_spectators:
            await ws.close(code=4003, reason="Spectating is disabled for this game")
            return
        conn.spectators.append(ws)
        logger.info("Spectator connected to game %s (total: %d)", game_id, len(conn.spectators))

    async def disconnect(self, game_id: str, ws: WebSocket) -> Optional[int]:
        """Remove a WebSocket.  Returns player_id if it was a player, else None."""
        conn = self._games.get(game_id)
        if conn is None:
            return None

        ws_id = id(ws)
        player_id = conn.ws_to_player.pop(ws_id, None)
        if player_id is not None and conn.players.get(player_id) is ws:
            del conn.players[player_id]
            logger.info("Player %d disconnected from game %s", player_id, game_id)
            return player_id

        if ws in conn.spectators:
            conn.spectators.remove(ws)
            logger.info("Spectator disconnected from game %s", game_id)

        return None

    def cleanup_game(self, game_id: str) -> None:
        """Remove all tracking for a game."""
        self._games.pop(game_id, None)

    # ------------------------------------------------------------------
    # Querying
    # ------------------------------------------------------------------

    def player_id_for(self, game_id: str, ws: WebSocket) -> Optional[int]:
        conn = self._games.get(game_id)
        if conn is None:
            return None
        return conn.ws_to_player.get(id(ws))

    def player_id_for_user(self, game_id: str, user_id: str) -> Optional[int]:
        """Return the player_id previously assigned to a user_id, if any."""
        conn = self._games.get(game_id)
        if conn is None:
            return None
        return conn.user_to_player.get(user_id)

    def player_count(self, game_id: str) -> int:
        conn = self._games.get(game_id)
        return len(conn.players) if conn else 0

    def spectator_count(self, game_id: str) -> int:
        conn = self._games.get(game_id)
        return len(conn.spectators) if conn else 0

    def has_game(self, game_id: str) -> bool:
        return game_id in self._games

    def next_available_slot(self, game_id: str) -> Optional[int]:
        """Return the next open player slot (1 or 2), or None if full."""
        conn = self._games.get(game_id)
        if conn is None:
            return 1
        if 1 not in conn.players:
            return 1
        if 2 not in conn.players:
            return 2
        return None

    # ------------------------------------------------------------------
    # Broadcasting
    # ------------------------------------------------------------------

    async def broadcast_state(
        self,
        game_id: str,
        runner: InteractiveGame,
        *,
        logs: Optional[List[str]] = None,
        events: Optional[List[Dict[str, Any]]] = None,
    ) -> None:
        """Send perspective-filtered state to every connected client."""
        conn = self._games.get(game_id)
        if conn is None:
            return

        full_state = runner.game.to_ui_json()
        mask = runner.get_action_mask().tolist()
        current_pid = runner.game.current_player_id
        descriptions = runner.game.describe_actions(current_pid)
        is_game_over = runner.is_game_over

        tasks: List[asyncio.Task] = []

        # Players get their own perspective
        for pid, ws in list(conn.players.items()):
            filtered = filter_state_for_player(full_state, pid)
            player_mask = mask if current_pid == pid else []
            payload: Dict[str, Any] = {
                "type": "state_update",
                "state": filtered,
                "action_mask": player_mask,
                "action_descriptions": descriptions if player_mask else {},
                "current_player_id": current_pid,
                "is_game_over": is_game_over,
            }
            if logs:
                payload["logs"] = logs
            if events:
                payload["events"] = events
            if is_game_over:
                payload["winner_id"] = runner.game.winner.player_id if runner.game.winner else None
            tasks.append(asyncio.create_task(self._safe_send(ws, payload)))

        # Spectators get redacted state, no mask
        if conn.spectators:
            spec_state = filter_state_for_spectator(full_state, conn.settings.spectator_mode)
            spec_payload: Dict[str, Any] = {
                "type": "state_update",
                "state": spec_state,
                "current_player_id": current_pid,
                "is_game_over": is_game_over,
            }
            if logs:
                spec_payload["logs"] = logs
            if events:
                spec_payload["events"] = events
            if is_game_over:
                spec_payload["winner_id"] = runner.game.winner.player_id if runner.game.winner else None
            for ws in list(conn.spectators):
                tasks.append(asyncio.create_task(self._safe_send(ws, spec_payload)))

        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)

    async def send_to_player(
        self, game_id: str, player_id: int, payload: Dict[str, Any]
    ) -> None:
        conn = self._games.get(game_id)
        if conn is None:
            return
        ws = conn.players.get(player_id)
        if ws is not None:
            await self._safe_send(ws, payload)

    async def broadcast_event(self, game_id: str, payload: Dict[str, Any]) -> None:
        """Send an event message (non-state) to all connected clients."""
        conn = self._games.get(game_id)
        if conn is None:
            return
        tasks = []
        for ws in list(conn.players.values()):
            tasks.append(asyncio.create_task(self._safe_send(ws, payload)))
        for ws in list(conn.spectators):
            tasks.append(asyncio.create_task(self._safe_send(ws, payload)))
        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    @staticmethod
    async def _safe_send(ws: WebSocket, data: Dict[str, Any]) -> None:
        try:
            if ws.client_state == WebSocketState.CONNECTED:
                await ws.send_json(data)
        except Exception:
            logger.debug("Failed to send to WebSocket", exc_info=True)


# Module-level singleton
manager = ConnectionManager()
