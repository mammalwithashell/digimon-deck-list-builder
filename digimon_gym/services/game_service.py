"""GameService: centralised game-session lifecycle management.

Eliminates the duplicated game-creation / step / state logic that was
spread across ``routers/games.py`` and ``desktop_main.py``.

Engine-only — no database, no auth, no AI pipeline imports.
Safe for both the hosted API and the desktop sidecar.
"""

from __future__ import annotations

import time
from typing import Any, Dict, List, Optional, Union
from uuid import uuid4

from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.data.deck_loader import parse_deck
from digimon_gym.engine.game.rules import Rules
from digimon_gym.engine.model_utils import resolve_model_path
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame


class GameService:
    """Manages in-memory game sessions with optional TTL eviction.

    Parameters
    ----------
    max_games:
        Maximum concurrent sessions.  When exceeded the oldest (by last
        access) is evicted.
    ttl_seconds:
        Idle sessions older than this are reaped on the next ``create_*``
        call.  ``0`` disables TTL eviction.
    """

    def __init__(self, *, max_games: int = 50, ttl_seconds: int = 0):
        self.max_games = max_games
        self.ttl_seconds = ttl_seconds
        self._games: Dict[str, Union[HeadlessGame, InteractiveGame]] = {}
        self._last_access: Dict[str, float] = {}

    # ── Session bookkeeping ───────────────────────────────────────────

    @property
    def active_games(self) -> Dict[str, Union[HeadlessGame, InteractiveGame]]:
        """Direct dict access for backward-compat with ``state.active_games``."""
        return self._games

    def _touch(self, game_id: str) -> None:
        self._last_access[game_id] = time.time()

    def _evict_stale(self) -> None:
        if self.ttl_seconds <= 0:
            return
        now = time.time()
        expired = [gid for gid, ts in self._last_access.items()
                   if now - ts > self.ttl_seconds]
        for gid in expired:
            self._games.pop(gid, None)
            self._last_access.pop(gid, None)

    def _evict_oldest(self) -> None:
        while len(self._games) >= self.max_games and self._last_access:
            oldest = min(self._last_access, key=self._last_access.get)  # type: ignore[arg-type]
            self._games.pop(oldest, None)
            self._last_access.pop(oldest, None)

    # ── Lookup ────────────────────────────────────────────────────────

    def get(self, game_id: str) -> Optional[Union[HeadlessGame, InteractiveGame]]:
        runner = self._games.get(game_id)
        if runner is not None:
            self._touch(game_id)
        return runner

    def delete(self, game_id: str) -> None:
        self._games.pop(game_id, None)
        self._last_access.pop(game_id, None)

    # ── Game creation ─────────────────────────────────────────────────

    def create_interactive_game(
        self,
        deck1: List[str],
        deck2: List[str],
        p1_type: PlayerType,
        p2_type: PlayerType,
        *,
        p1_policy: str = "greedy",
        p2_policy: str = "greedy",
        agent_action_delay_ms: int = 350,
        p1_model_path: Optional[str] = None,
        p2_model_path: Optional[str] = None,
        rules: Optional[Rules] = None,
    ) -> tuple[str, InteractiveGame]:
        """Create an interactive (human/agent) game and register it.

        Returns ``(game_id, runner)``.
        """
        self._evict_stale()
        self._evict_oldest()

        game_id = str(uuid4())
        runner = InteractiveGame(
            deck1, deck2, p1_type, p2_type,
            player1_policy=p1_policy,
            player2_policy=p2_policy,
            agent_action_delay_ms=agent_action_delay_ms,
            player1_model_path=p1_model_path,
            player2_model_path=p2_model_path,
            rules=rules,
        )
        self._games[game_id] = runner
        self._touch(game_id)
        runner.clear_log()
        return game_id, runner

    def create_headless_game(
        self,
        deck1: List[str],
        deck2: List[str],
        *,
        verbose: bool = True,
        record_actions: bool = False,
        record_tensors: bool = False,
        rules: Optional[Rules] = None,
    ) -> tuple[str, HeadlessGame]:
        """Create a headless (agent-vs-agent) game and register it.

        Returns ``(game_id, runner)``.
        """
        self._evict_stale()
        self._evict_oldest()

        game_id = str(uuid4())
        runner = HeadlessGame(
            deck1, deck2,
            verbose=verbose,
            record_actions=record_actions,
            record_tensors=record_tensors,
            rules=rules,
        )
        self._games[game_id] = runner
        self._touch(game_id)
        return game_id, runner
