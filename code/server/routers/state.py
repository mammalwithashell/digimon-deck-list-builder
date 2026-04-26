"""In-memory session stores shared across API routers."""

from __future__ import annotations

from engine_py_legacy.engine.runners.headless_game import HeadlessGame
from engine_py_legacy.engine.runners.interactive_game import InteractiveGame
from engine_py_legacy.engine.runners.replay_runner import ReplayRunner

active_games: dict[str, HeadlessGame | InteractiveGame] = {}
active_replays: dict[str, ReplayRunner] = {}

