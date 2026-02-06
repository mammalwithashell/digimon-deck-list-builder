"""Game logger abstraction for headless and interactive game modes.

Mirrors C# Digimon.Core.Loggers: IGameLogger, SilentLogger, VerboseLogger.
"""

from abc import ABC, abstractmethod
from typing import List


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
