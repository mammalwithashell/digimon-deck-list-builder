"""`digivolve_driven_attack` component — reward Lv5+ attacks on security
when the attacker came from a digivolve chain (sources) or was just
digivolved this turn.

Spec:
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
`digivolve_driven_attack component`.

The engine increments `n_digivolve_driven_attacks[player]` once per
qualifying attack (Lv5+ attacker, security target, attack actually
connects — see the `engine-event-emission` capability). The bus
materializes one `DigivolveDrivenAttack` occurrence per unit delta with
the `has_sources` / `this_turn` flags read from the attacker's
permanent state.

The component then filters per-occurrence on:
- `attacker_level >= attacker_min_level` (engine already filters at 5;
  the component re-checks against its own — possibly higher — threshold).
- A mode predicate over the (this_turn, has_sources) flags:
    `this_turn`    : event.this_turn
    `has_sources`  : event.has_sources
    `either`       : event.this_turn OR event.has_sources
    `both`         : event.this_turn AND event.has_sources

`per_card` is a v1-deferred parameter. When set True, a warning is
logged at construction; behavior is the same as `per_card=false`.

Per-agent only (`player == 1`, Python convention).
"""

from __future__ import annotations

import warnings
from dataclasses import dataclass, field
from typing import Any, MutableMapping, Sequence

from ..occurrences import DigivolveDrivenAttack


_VALID_MODES = frozenset({"this_turn", "has_sources", "either", "both"})


@dataclass
class DigivolveDrivenAttackComponent:
    """Per-event level + mode-flag filter; emits `reward` per match."""

    name: str
    mode: str = "either"
    attacker_min_level: int = 5
    reward: float = 0.5
    per_card: bool = False
    # Cached because the warning should fire at construction (not on
    # every compute) when per_card is True. Set by __post_init__.
    _per_card_warned: bool = field(default=False, init=False, repr=False)

    def __post_init__(self) -> None:
        if self.mode not in _VALID_MODES:
            raise ValueError(
                f"digivolve_driven_attack: invalid mode {self.mode!r}; "
                f"must be one of {sorted(_VALID_MODES)}"
            )
        if self.per_card and not self._per_card_warned:
            warnings.warn(
                f"digivolve_driven_attack component {self.name!r}: "
                "`per_card: true` is a v2 feature — per-card semantics "
                "are not implemented in v1. Behaving as `per_card: false` "
                "(one fire per attack regardless of Security Attack +N).",
                UserWarning,
                stacklevel=2,
            )
            self._per_card_warned = True

    def _mode_matches(self, ev: DigivolveDrivenAttack) -> bool:
        if self.mode == "this_turn":
            return bool(ev.this_turn)
        if self.mode == "has_sources":
            return bool(ev.has_sources)
        if self.mode == "either":
            return bool(ev.this_turn) or bool(ev.has_sources)
        # self.mode == "both"
        return bool(ev.this_turn) and bool(ev.has_sources)

    def compute(
        self,
        occurrences: Sequence[Any],
        episode_state: MutableMapping[str, Any],
    ) -> float:
        total = 0.0
        for ev in occurrences:
            if not isinstance(ev, DigivolveDrivenAttack):
                continue
            if ev.player != 1:
                continue
            if ev.attacker_level < self.attacker_min_level:
                continue
            if not self._mode_matches(ev):
                continue
            total += float(self.reward)
        return total
