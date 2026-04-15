"""Application services layer.

Shared game-session management consumed by API routers and the desktop sidecar.
No database or auth dependencies — safe for all deployment targets.
"""

from .game_service import GameService

__all__ = ["GameService"]
