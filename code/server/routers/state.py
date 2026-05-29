"""In-memory session stores shared across API routers.

`active_games` is the cross-router registry of in-flight game sessions.
With the Rust-engine cutover in `games.py`, the held runner is
`digimon_engine.RustHeadlessGame`. Other routers (replays, etc) still
register their own session types here under different ids; the dict is
typed as `object` so it tolerates the mixed shape — each consumer
should isinstance-check before using.
"""

from __future__ import annotations

from typing import Any

from engine_py_legacy.engine.runners.replay_runner import ReplayRunner

active_games: dict[str, Any] = {}
active_replays: dict[str, ReplayRunner] = {}

