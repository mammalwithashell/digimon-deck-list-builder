"""`breeding_digivolve` component — per-level reward for breeding-area raises.

Spec:
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`breeding_digivolve component`.

Fires on `Digivolved` occurrences flagged `is_breeding=true` (the bus
sets the flag when `field_index == BREEDING_TARGET`). Per-agent only
(`player == 1`, Python convention).

`reward_per_level` is a dict mapping the post-digivolve level to its
reward. Default shipped values:
    {3: 0.4, 4: 0.2, 5: 0.1, 6: -0.4}
- Lv3 (from a Lv2 egg) = highest reward — cleans the slot and feeds the
  next egg.
- Each subsequent level halves the reward.
- Lv6 is the explicit anti-pattern: digivolving to Lv6 in breeding
  locks the slot (Lv6 can't easily be played onto the field with proper
  memory cost). Same magnitude as Lv3 but negative.

Result levels not in the dict produce zero.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, Mapping, MutableMapping, Sequence

from ..occurrences import Digivolved


@dataclass
class BreedingDigivolveComponent:
    """Per-result-level reward lookup for breeding-area digivolves."""

    name: str
    reward_per_level: Mapping[int, float] = field(default_factory=dict)

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        total = 0.0
        for ev in occurrences:
            if not isinstance(ev, Digivolved):
                continue
            if ev.player != 1:
                continue
            if not ev.is_breeding:
                continue
            if ev.result_level is None:
                continue
            # Dict miss → zero (operators can omit levels they don't
            # want to reward).
            total += float(self.reward_per_level.get(int(ev.result_level), 0.0))
        return total
