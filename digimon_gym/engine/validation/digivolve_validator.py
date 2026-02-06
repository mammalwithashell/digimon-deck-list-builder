"""Digivolution validation logic.

Provides can_digivolve() which checks whether a card from hand can legally
digivolve onto a permanent on the field, using the card's evo_costs data.

Also provides DNA Digivolution validation via can_dna_digivolve() and
helper functions for checking whether valid DNA targets exist.
"""

from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional, Tuple

if TYPE_CHECKING:
    from ..core.card_source import CardSource
    from ..core.permanent import Permanent
    from ..data.evo_cost import DnaCost, DnaRequirement


def can_digivolve(evo_card: 'CardSource', base_perm: 'Permanent') -> bool:
    """Check if evo_card can legally digivolve onto base_perm using evo_costs.

    Rules:
    1. evo_card must be a Digimon with non-empty evo_costs
    2. At least one evo_cost entry must match:
       - evo_cost.level == base_perm.level (base must be at the required level)
       - evo_cost.card_color in base_perm.top_card.card_colors (base must have the required color)

    Note: Memory cost is NOT checked here. Digivolution can send memory
    negative (triggering a turn switch), unlike playing cards.

    Args:
        evo_card: The card from hand that would digivolve onto the base.
        base_perm: The permanent on the field being digivolved onto.

    Returns:
        True if at least one evo_cost requirement is satisfied.
    """
    if not evo_card.is_digimon:
        return False

    if not evo_card.c_entity_base or not evo_card.c_entity_base.evo_costs:
        return False

    if not base_perm.top_card:
        return False

    base_colors = set(base_perm.top_card.card_colors)
    base_level = base_perm.level

    for evo_cost in evo_card.c_entity_base.evo_costs:
        if evo_cost.level == base_level and evo_cost.card_color in base_colors:
            return True

    return False


# ─── DNA Digivolution Validation ──────────────────────────────────────


def _perm_matches_dna_req(perm: 'Permanent', req: 'DnaRequirement') -> bool:
    """Check if a permanent satisfies a single DNA requirement half.

    Checks level, color (if specified), and name (if specified).
    """
    if not perm.top_card:
        return False

    # Level must match
    if perm.level != req.level:
        return False

    # Color must match (if specified)
    if req.card_color is not None:
        if req.card_color not in perm.top_card.card_colors:
            return False

    # Name must match (if specified)
    if req.name_contains:
        if not perm.contains_card_name(req.name_contains):
            return False

    return True


def can_dna_digivolve(evo_card: 'CardSource',
                      perm1: 'Permanent',
                      perm2: 'Permanent') -> bool:
    """Check if evo_card can DNA digivolve using perm1 and perm2.

    Tests both orderings: (perm1=req1, perm2=req2) and (perm1=req2, perm2=req1).
    Both permanents must be distinct.

    Args:
        evo_card: The card from hand with DNA digivolution requirements.
        perm1: First candidate permanent on the field.
        perm2: Second candidate permanent on the field.

    Returns:
        True if any dna_cost requirement is satisfied by the two permanents.
    """
    if not evo_card.is_digimon:
        return False

    if not evo_card.c_entity_base or not evo_card.c_entity_base.dna_costs:
        return False

    if perm1 is perm2:
        return False

    for dna_cost in evo_card.c_entity_base.dna_costs:
        # Try both orderings
        if (_perm_matches_dna_req(perm1, dna_cost.requirement1) and
                _perm_matches_dna_req(perm2, dna_cost.requirement2)):
            return True
        if (_perm_matches_dna_req(perm1, dna_cost.requirement2) and
                _perm_matches_dna_req(perm2, dna_cost.requirement1)):
            return True

    return False


def get_dna_stacking_order(
    evo_card: 'CardSource',
    perm1: 'Permanent',
    perm2: 'Permanent',
) -> Optional[Tuple['Permanent', 'Permanent', 'DnaCost']]:
    """Determine the stacking order for DNA digivolution.

    Returns (top_perm, bottom_perm, dna_cost) where:
    - top_perm matches requirement1 (first listed, sources go on TOP)
    - bottom_perm matches requirement2 (second listed, sources go on BOTTOM)

    Returns None if the combination is invalid.
    """
    if not evo_card.c_entity_base or not evo_card.c_entity_base.dna_costs:
        return None

    for dna_cost in evo_card.c_entity_base.dna_costs:
        # perm1 = req1, perm2 = req2
        if (_perm_matches_dna_req(perm1, dna_cost.requirement1) and
                _perm_matches_dna_req(perm2, dna_cost.requirement2)):
            return (perm1, perm2, dna_cost)
        # perm1 = req2, perm2 = req1
        if (_perm_matches_dna_req(perm1, dna_cost.requirement2) and
                _perm_matches_dna_req(perm2, dna_cost.requirement1)):
            return (perm2, perm1, dna_cost)

    return None


def has_valid_dna_targets(evo_card: 'CardSource',
                          battle_area: List['Permanent']) -> bool:
    """Check if there exist two permanents in the battle area that satisfy
    any of the card's DNA digivolution requirements."""
    if not evo_card.is_digimon:
        return False
    if not evo_card.c_entity_base or not evo_card.c_entity_base.dna_costs:
        return False

    for i in range(len(battle_area)):
        for j in range(i + 1, len(battle_area)):
            if can_dna_digivolve(evo_card, battle_area[i], battle_area[j]):
                return True
    return False


def get_valid_dna_first_targets(
    evo_card: 'CardSource',
    battle_area: List['Permanent'],
) -> List[int]:
    """Return field indices of permanents that could be the first DNA target.

    A permanent is a valid first target if there exists at least one other
    permanent in the battle area that together satisfies a DNA requirement.
    """
    valid = []
    for i in range(len(battle_area)):
        for j in range(len(battle_area)):
            if i != j and can_dna_digivolve(evo_card, battle_area[i], battle_area[j]):
                valid.append(i)
                break
    return valid


def get_valid_dna_second_targets(
    evo_card: 'CardSource',
    first_idx: int,
    battle_area: List['Permanent'],
) -> List[int]:
    """Return field indices of permanents that can be the second DNA target,
    given the first target has already been selected."""
    if first_idx >= len(battle_area):
        return []
    first_perm = battle_area[first_idx]
    valid = []
    for j in range(len(battle_area)):
        if j != first_idx and can_dna_digivolve(evo_card, first_perm, battle_area[j]):
            valid.append(j)
    return valid
