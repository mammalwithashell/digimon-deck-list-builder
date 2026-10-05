"""Per-run vendor availability (spec "Failures are retried by class" / task 3.6).

Once a family reports quota or credit exhaustion it is disabled for the rest of
the run: `run_with_retry` stops calling it, and the router / driver consult
`is_available` (or subscribe) to re-route or pause that family's pending items.
A disabled family is never re-enabled within a run; a new run starts healthy.
"""
from __future__ import annotations

import threading
import time
from typing import Callable

from ..contracts import FAMILIES

Listener = Callable[[str, str], None]


class VendorHealth:
    def __init__(self, *, clock: Callable[[], float] = time.time):
        self._clock = clock
        self._lock = threading.Lock()
        self._disabled: dict[str, dict] = {}
        self._listeners: list[Listener] = []

    def disable(self, family: str, reason: str) -> bool:
        """Disable `family`; returns True on the first disable (listeners fire once)."""
        if family not in FAMILIES:
            raise ValueError(f"unknown family {family!r}")
        with self._lock:
            if family in self._disabled:
                return False
            self._disabled[family] = {"reason": reason, "at": self._clock()}
            listeners = list(self._listeners)
        for fn in listeners:
            fn(family, reason)
        return True

    def is_available(self, family: str) -> bool:
        if family not in FAMILIES:
            raise ValueError(f"unknown family {family!r}")
        with self._lock:
            return family not in self._disabled

    def reason(self, family: str) -> str | None:
        with self._lock:
            entry = self._disabled.get(family)
            return entry["reason"] if entry else None

    def available(self) -> tuple[str, ...]:
        with self._lock:
            return tuple(f for f in FAMILIES if f not in self._disabled)

    def subscribe(self, fn: Listener) -> None:
        """`fn(family, reason)` is called once when a family is disabled."""
        with self._lock:
            self._listeners.append(fn)

    def snapshot(self) -> dict:
        """JSON-able state for events.jsonl / run reports."""
        with self._lock:
            return {f: dict(v) for f, v in self._disabled.items()}
