"""`digivolve_into_named_card` component — the arrival-aware mirror of
`play_named_card`, gated on digivolve-specific flags.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"v1 component catalog" entry 8 + scenarios "digivolve_into_named_card
with was_dna gating" and "digivolve_into_named_card with min_result_level".

Match keys (at least one required; multiple ANDed together):
- `card_id`     — exact result-card-ID match. String or list.
- `card_name`   — exact result-card-name match. (Enrichment TBD — see
                  `play.py` for the same documented limitation.)
- `trait`       — membership in the result card's trait list. Consumes
                  `Digivolved.result_traits` (bus-enriched).

Optional gating keys (all SHALL match when set; ANDed):
- `was_dna`             — bool: only DNA digivolves (true), only non-DNA
                          (false), or any (None / unset).
- `was_blast_dna`       — bool: same shape, narrower.
- `min_result_level`    — int: only digivolves whose result level >= N.
                          Consumes `Digivolved.result_level`
                          (bus-enriched).

This component drives the central DNA Omnimon use case: rewarding
"DNA-digivolved INTO Omnimon" while a sibling `play_named_card` with
`cost_paid_gte_printed: true` penalizes hardcasting Omnimon.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, List, MutableMapping, Optional, Sequence

from ..occurrences import Digivolved


@dataclass
class DigivolveIntoNamedCardComponent:
    """Emits `weight` for each agent-side `Digivolved` event whose
    result card matches AND whose DNA / level gating satisfies. The
    budget engine handles per-episode cap and diminishing returns.
    """

    name: str
    weight: float
    # Match keys.
    match_card_id: Optional[List[str]] = None
    match_card_name: Optional[List[str]] = None  # enrichment TBD
    match_trait: Optional[List[str]] = None
    # Gating keys.
    was_dna: Optional[bool] = None
    was_blast_dna: Optional[bool] = None
    min_result_level: Optional[int] = None

    def _matches(self, ev: Digivolved) -> bool:
        any_match_set = (
            self.match_card_id is not None
            or self.match_card_name is not None
            or self.match_trait is not None
        )
        if not any_match_set:
            return False

        if self.match_card_id is not None and ev.top_card_id not in self.match_card_id:
            return False
        if self.match_card_name is not None:
            # Same enrichment limitation as `play_named_card`.
            return False
        if self.match_trait is not None:
            # Consumes bus-enriched `result_traits`. Membership = any-of.
            if not any(t in ev.result_traits for t in self.match_trait):
                return False

        # Gating.
        if self.was_dna is not None and ev.was_dna != self.was_dna:
            return False
        if self.was_blast_dna is not None and ev.was_blast_dna != self.was_blast_dna:
            return False
        if self.min_result_level is not None:
            if ev.result_level is None or ev.result_level < self.min_result_level:
                return False

        return True

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        count = 0
        for ev in occurrences:
            if not isinstance(ev, Digivolved):
                continue
            if ev.player != 1:
                continue  # agent-side only
            if self._matches(ev):
                count += 1
        return float(self.weight) * float(count) if count else 0.0
