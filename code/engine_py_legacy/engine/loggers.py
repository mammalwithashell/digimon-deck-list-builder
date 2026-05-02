"""Game logger abstraction for headless and interactive game modes.

Mirrors C# Digimon.Core.Loggers: IGameLogger, SilentLogger, VerboseLogger.
EventLogger extends VerboseLogger with structured GameEvent support.
"""

from abc import ABC, abstractmethod
from typing import Dict, List, Any

from engine_py_legacy.engine.events import GameEvent


class IGameLogger(ABC):
    """Abstract base for game logging."""

    @abstractmethod
    def log(self, message: str) -> None:
        ...

    @abstractmethod
    def log_verbose(self, message: str) -> None:
        ...

    @abstractmethod
    def get_logs(self) -> List[str]:
        ...

    @abstractmethod
    def clear(self) -> None:
        ...


class SilentLogger(IGameLogger):
    """No-op logger for maximum performance in RL training."""

    def log(self, message: str) -> None:
        pass

    def log_verbose(self, message: str) -> None:
        pass

    def get_logs(self) -> List[str]:
        return []

    def clear(self) -> None:
        pass


class VerboseLogger(IGameLogger):
    """Buffers all log messages for retrieval. Used by InteractiveGame."""

    def __init__(self):
        self._logs: List[str] = []

    def log(self, message: str) -> None:
        self._logs.append(message)

    def log_verbose(self, message: str) -> None:
        self._logs.append(f"[VERBOSE] {message}")

    def get_logs(self) -> List[str]:
        return list(self._logs)

    def clear(self) -> None:
        self._logs.clear()


class EventLogger(VerboseLogger):
    """VerboseLogger + structured GameEvent buffer for interactive games."""

    def __init__(self):
        super().__init__()
        self._events: List[GameEvent] = []

    def emit(self, event: GameEvent) -> None:
        self._events.append(event)

    def get_events(self) -> List[Dict[str, Any]]:
        return [e.to_dict() for e in self._events]

    def clear_events(self) -> None:
        self._events.clear()

    def clear(self) -> None:
        super().clear()
        self._events.clear()
