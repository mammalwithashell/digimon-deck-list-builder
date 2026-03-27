"""In-memory session stores shared across API routers."""

from __future__ import annotations

from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.runners.replay_runner import ReplayRunner
from digimon_gym.services.game_service import GameService

# Primary game session manager (shared across routers)
game_service = GameService(max_games=50, ttl_seconds=0)

# Backward-compatible alias — existing code that reads/writes active_games
# directly (e.g. WS routers) continues to work via the service's internal dict.
active_games: dict[str, HeadlessGame | InteractiveGame] = game_service.active_games

active_replays: dict[str, ReplayRunner] = {}
