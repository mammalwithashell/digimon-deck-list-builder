"""`memory_swing` component.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"v1 component catalog" entry 9.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import MemoryShifted


@dataclass
class MemorySwingComponent:
    """Emits `weight × delta` per step where `delta` is signed memory
    movement toward the agent's side this step. The bus emits a single
    aggregated `MemoryShifted` per step (sum of all per-event deltas).

    `weight` is conventionally positive — memory swinging toward the
    agent is good. The signed `delta` carries the direction.
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        total_delta = sum(ev.delta for ev in occurrences if isinstance(ev, MemoryShifted))
        return float(self.weight) * float(total_delta) if total_delta else 0.0
