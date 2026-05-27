"""`digivolve` and `dna_digivolve` components — the simple counter-style
digivolve rewards.

For arrival-aware "digivolve INTO a specific card" matching, see
`components/digivolve_into.py`.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"v1 component catalog" entries 5 + 6.

Per spec decision 5 (digivolve shaping precedent): DNA digivolves stack
on the regular digivolve counter. The `RewardEventBus` emits BOTH a
`Digivolved` AND a `DnaDigivolved` occurrence for one DNA digivolve, so
a profile that wires both `digivolve` and `dna_digivolve` components
fires both. This is intentional — operators who want only the DNA
bonus can set `digivolve.weight = 0`.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, MutableMapping, Sequence

from ..occurrences import Digivolved, DnaDigivolved


@dataclass
class DigivolveComponent:
    """Emits `weight × n` per step where `n` counts the agent's
    digivolutions this step (regardless of DNA / non-DNA). Mirrors the
    engine's `n_digivolutions` counter semantic (which also bumps on
    DNA per the 2026-05-23 design).
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        count = sum(1 for ev in occurrences if isinstance(ev, Digivolved) and ev.player == 1)
        return float(self.weight) * float(count) if count else 0.0


@dataclass
class DnaDigivolveComponent:
    """Emits `weight × n` per step where `n` counts the agent's DNA
    digivolutions this step. Operators typically set this in addition
    to (not instead of) `digivolve` — the DNA reward stacks on top.
    """

    name: str
    weight: float

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        count = sum(1 for ev in occurrences if isinstance(ev, DnaDigivolved) and ev.player == 1)
        return float(self.weight) * float(count) if count else 0.0
