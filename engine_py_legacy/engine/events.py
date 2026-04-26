"""Structured game events for rich frontend feedback.

Emitted alongside string logs by EventLogger. SilentLogger ignores these
entirely — zero overhead for RL training.
"""

from __future__ import annotations

from dataclasses import dataclass, field, asdict
from typing import Any, Dict, Optional


@dataclass(slots=True)
class GameEvent:
    """A single structured game event."""

    type: str
    seq: int
    player: int = 0
    source_card_id: Optional[str] = None
    source_slot: Optional[int] = None
    target_card_id: Optional[str] = None
    target_slot: Optional[int] = None
    meta: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)
