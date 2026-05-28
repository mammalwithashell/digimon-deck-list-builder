"""`block_event`, `opp_deletion`, `own_deletion` — combat-event-driven
shaping signals.

Requires the `engine-event-emission` capability (wired in Group 1 of
the `add-reward-profiles` change). `Blocked`, `OppDeleted`, and
`OwnDeleted` occurrences are derived from `GameEvent::Attack` (with
block resolution lookup) and `GameEvent::Trash` (filtered to
battle-area-origin events).

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"v1 component catalog" entries 10 + 11 + 12.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import Blocked, OppDeleted, OwnDeleted


@dataclass
class BlockEventComponent:
    """Emits `weight × n` per step where `n` counts attacks the agent
    blocked this step. `Blocked.blocker_player == 1` filters for
    agent-side blocks.
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        count = sum(1 for ev in occurrences if isinstance(ev, Blocked) and ev.blocker_player == 1)
        return float(self.weight) * float(count) if count else 0.0


@dataclass
class OppDeletionComponent:
    """Emits `weight × n` per step where `n` counts opponent Digimon
    deleted (battle-area → trash) this step. `owner_player == 2`
    filters for opponent-side deletions.
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        count = sum(1 for ev in occurrences if isinstance(ev, OppDeleted) and ev.owner_player == 2)
        return float(self.weight) * float(count) if count else 0.0


@dataclass
class OwnDeletionComponent:
    """Mirror of `OppDeletionComponent` for own-side losses. `weight`
    is conventionally negative — losing your own Digimon is bad.
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        count = sum(1 for ev in occurrences if isinstance(ev, OwnDeleted) and ev.owner_player == 1)
        return float(self.weight) * float(count) if count else 0.0
