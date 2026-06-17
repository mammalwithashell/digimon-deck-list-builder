"""In-memory session stores shared across API routers.

`active_games` is the cross-router registry of in-flight game sessions.
With the Rust-engine cutover in `games.py`, the held runner is
`digimon_engine.RustHeadlessGame`. `active_replays` holds
`digimon_engine.RustReplayRunner` sessions. Both dicts are typed as
`Any` so they tolerate the mixed shape — each consumer should
isinstance-check before using.
"""

from __future__ import annotations

from typing import Any

active_games: dict[str, Any] = {}
active_replays: dict[str, Any] = {}

