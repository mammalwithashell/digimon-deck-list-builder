"""In-memory session stores shared across API routers."""

from __future__ import annotations

from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.runners.replay_runner import ReplayRunner

active_games: dict[str, HeadlessGame | InteractiveGame] = {}
active_replays: dict[str, ReplayRunner] = {}

