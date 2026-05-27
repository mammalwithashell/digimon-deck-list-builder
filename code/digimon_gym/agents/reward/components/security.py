"""`security_remove` and `security_lost` components.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"v1 component catalog" entries 3 + 4, plus scenario
"security_remove fires only on the step security count drops".
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import SecurityLost, SecurityRemoved


@dataclass
class SecurityRemoveComponent:
    """Emits `weight × n` per step where `n` is the count of opponent
    security cards removed this step. The `RewardEventBus` derives a
    `SecurityRemoved` occurrence only when the count actually dropped,
    so this component returns 0 on steps with no removal.
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        total = 0
        for ev in occurrences:
            if isinstance(ev, SecurityRemoved):
                total += ev.count
        return float(self.weight) * float(total) if total else 0.0


@dataclass
class SecurityLostComponent:
    """Mirror of `SecurityRemoveComponent` for own-side security loss.
    `weight` is conventionally negative to penalize losing security.
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        total = 0
        for ev in occurrences:
            if isinstance(ev, SecurityLost):
                total += ev.count
        return float(self.weight) * float(total) if total else 0.0
