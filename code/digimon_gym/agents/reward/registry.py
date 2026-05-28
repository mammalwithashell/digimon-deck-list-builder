"""Component registry — maps each `kind` string (as used in
`profiles.yaml`) to its implementation class.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"Reward components are composable units with a uniform interface" —
"Loading a profile that references an unknown `kind` SHALL fail at
load time with an error listing the registered kinds."

The profile loader (`profile_loader.py`, Group 4) consults this map
when materializing each component declaration from YAML. The registry
also encodes the **key parameters** per kind (which fields participate
in inheritance-override de-duplication per spec D3 + D15).
"""

from __future__ import annotations

from typing import Dict, FrozenSet, Type

from .component import RewardComponent
from .components.breeding import BreedingDigivolveComponent
from .components.combat import (
    BlockEventComponent,
    OppDeletionComponent,
    OwnDeletionComponent,
)
from .components.digivolve import DigivolveComponent, DnaDigivolveComponent
from .components.digivolve_driven_attack import DigivolveDrivenAttackComponent
from .components.digivolve_into import DigivolveIntoNamedCardComponent
from .components.match_outcome import MatchOutcomeComponent
from .components.memory import MemorySwingComponent
from .components.play import PlayNamedCardComponent
from .components.quick_win import QuickWinBonusComponent
from .components.security import SecurityLostComponent, SecurityRemoveComponent
from .components.stall import StallPenaltyComponent
from .components.terminal import StepPenaltyComponent, TerminalOutcomeComponent


# kind -> implementation class. Stable string keys exposed to operators
# via YAML `kind: <name>`. Adding a new component kind requires:
#   1. Implement the class in `components/`.
#   2. Register here.
#   3. If the kind allows multiple instances per profile differentiated
#      by parameters, list those param names in `KIND_KEY_PARAMETERS`.
COMPONENT_REGISTRY: Dict[str, Type[RewardComponent]] = {
    "terminal_outcome": TerminalOutcomeComponent,
    "step_penalty": StepPenaltyComponent,
    "security_remove": SecurityRemoveComponent,
    "security_lost": SecurityLostComponent,
    "digivolve": DigivolveComponent,
    "dna_digivolve": DnaDigivolveComponent,
    "play_named_card": PlayNamedCardComponent,
    "digivolve_into_named_card": DigivolveIntoNamedCardComponent,
    "memory_swing": MemorySwingComponent,
    "block_event": BlockEventComponent,
    "opp_deletion": OppDeletionComponent,
    "own_deletion": OwnDeletionComponent,
    # add-gameplay-reward-config additions:
    "quick_win_bonus": QuickWinBonusComponent,
    "stall_penalty": StallPenaltyComponent,
    "breeding_digivolve": BreedingDigivolveComponent,
    "digivolve_driven_attack": DigivolveDrivenAttackComponent,
    # match_outcome — Path C extension: per-BO3-match terminal reward.
    # When in the active profile, MatchEnv defers its hardcoded match-tier
    # constants AND its per-game (bo3 − inner) adjustment to this component.
    "match_outcome": MatchOutcomeComponent,
}


# Per-kind key parameters for inheritance override (spec D3 + D15).
# A child profile's component REPLACES a parent's matching component
# when (kind, key_params) tuples match. Most components are
# single-instance (empty frozenset → at most one per profile).
# `play_named_card` and `digivolve_into_named_card` allow multiple
# instances differentiated by their match/gating fields.
KIND_KEY_PARAMETERS: Dict[str, FrozenSet[str]] = {
    "terminal_outcome": frozenset(),
    "step_penalty": frozenset(),
    "security_remove": frozenset(),
    "security_lost": frozenset(),
    "digivolve": frozenset(),
    "dna_digivolve": frozenset(),
    "memory_swing": frozenset(),
    "block_event": frozenset(),
    "opp_deletion": frozenset(),
    "own_deletion": frozenset(),
    # add-gameplay-reward-config: all single-instance components.
    "quick_win_bonus": frozenset(),
    "stall_penalty": frozenset(),
    "breeding_digivolve": frozenset(),
    "digivolve_driven_attack": frozenset(),
    "match_outcome": frozenset(),
    # Distinct instances per match/gating combination. The loader
    # serializes these to a canonical key tuple for override matching.
    "play_named_card": frozenset(
        {
            "match",
            "cost_paid_lt",
            "cost_paid_eq",
            "cost_paid_gte",
            "cost_paid_lt_printed",
            "cost_paid_gte_printed",
            "via_alt_path",
        }
    ),
    "digivolve_into_named_card": frozenset(
        {"match", "was_dna", "was_blast_dna", "min_result_level"}
    ),
}


class UnknownComponentKindError(ValueError):
    """Raised when `profiles.yaml` references a kind not in
    `COMPONENT_REGISTRY`. The message lists every registered kind.
    """

    def __init__(self, kind: str) -> None:
        registered = ", ".join(sorted(COMPONENT_REGISTRY))
        super().__init__(
            f"Unknown reward component kind: {kind!r}. "
            f"Registered kinds: {registered}"
        )
        self.kind = kind


def get_component_class(kind: str) -> Type[RewardComponent]:
    """Look up the implementation class for a registered kind.

    Raises `UnknownComponentKindError` if the kind is not registered,
    with a message that includes every valid kind. This is the
    spec-mandated error surface for the "Unknown component kind fails
    at load" scenario.
    """
    try:
        return COMPONENT_REGISTRY[kind]
    except KeyError:
        raise UnknownComponentKindError(kind) from None
