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
    """Check if evo_card can legally digivolve onto base_perm.

    Checks two sources of digivolution requirements:
    1. Standard evo_costs from card metadata (level + color match)
    2. Alternate digivolution from card script effects (_alt_digi_* fields)

    Note: Memory cost is NOT checked here. Digivolution can send memory
    negative (triggering a turn switch), unlike playing cards.

    Args:
        evo_card: The card from hand that would digivolve onto the base.
        base_perm: The permanent on the field being digivolved onto.

    Returns:
        True if at least one evo requirement (standard or alt) is satisfied.
    """
    if not evo_card.is_digimon:
        return False

    if not base_perm.top_card:
        return False

    # 1. Standard evo_costs check
    if evo_card.c_entity_base and evo_card.c_entity_base.evo_costs:
        base_colors = set(base_perm.top_card.card_colors)
        base_level = base_perm.level
        for evo_cost in evo_card.c_entity_base.evo_costs:
            if evo_cost.level == base_level and evo_cost.card_color in base_colors:
                return True

    # 2. Alternate digivolution check (from card script effects)
    if _check_alt_digivolve(evo_card, base_perm):
        return True

    return False


def _check_alt_digivolve(evo_card: 'CardSource', base_perm: 'Permanent') -> bool:
    """Check if evo_card has alt digi effects matching base_perm.

    Scans the evo_card's NoTiming effects for _alt_digi_* fields
    (set by the transpiler from AddSelfDigivolutionRequirementStaticEffect).
    """
    from ..data.enums import EffectTiming
    effects = evo_card.effect_list(EffectTiming.NoTiming)
    for effect in effects:
        cost = getattr(effect, '_alt_digi_cost', None)
        if cost is None:
            continue
        # Level check
        req_level = getattr(effect, '_alt_digi_level', None)
        if req_level is not None and base_perm.level != req_level:
            continue
        # Trait check
        req_trait = getattr(effect, '_alt_digi_trait', None)
        if req_trait is not None:
            traits = getattr(base_perm.top_card, 'card_traits', []) or []
            if not any(req_trait in t for t in traits):
                continue
        # Name check
        req_name = getattr(effect, '_alt_digi_name', None)
        if req_name is not None:
            if not base_perm.contains_card_name(req_name):
                continue
        # Condition check (e.g., "while you have [Owen Dreadnought]")
        if effect.can_use_condition is not None:
            ctx = {}
            if not effect.can_use_condition(ctx):
                continue
        return True
    return False


def get_alt_digi_cost(evo_card: 'CardSource', base_perm: 'Permanent') -> int:
    """Return the memory cost of the matching alt digi requirement, or 0."""
    from ..data.enums import EffectTiming

    effects = evo_card.effect_list(EffectTiming.NoTiming)
    for effect in effects:
        cost = getattr(effect, '_alt_digi_cost', None)
        if cost is None:
            continue
        req_level = getattr(effect, '_alt_digi_level', None)
        if req_level is not None and base_perm.level != req_level:
            continue
        req_trait = getattr(effect, '_alt_digi_trait', None)
        if req_trait is not None:
            traits = getattr(base_perm.top_card, 'card_traits', []) or []
            if not any(req_trait in t for t in traits):
                continue
        req_name = getattr(effect, '_alt_digi_name', None)
        if req_name is not None:
            if not base_perm.contains_card_name(req_name):
                continue
        # Condition check (e.g., "while you have [Owen Dreadnought]")
        if effect.can_use_condition is not None:
            ctx = {}
            if not effect.can_use_condition(ctx):
                continue
        return cost
    return 0


# ─── DNA Digivolution Validation ──────────────────────────────────────


def _perm_matches_dna_req(perm: 'Permanent', req: 'DnaRequirement') -> bool:
    """Check if a permanent satisfies a single DNA requirement half.

    Checks level, color (if specified), name (if specified), and
    effect text (if specified).
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

    # Effect text must match (if specified)
    if req.text_contains:
        entity = perm.top_card.c_entity_base
        if not entity:
            return False
        full_text = (
            entity.effect_description_eng
            + entity.inherited_effect_description_eng
            + entity.security_effect_description_eng
        )
        if req.text_contains.lower() not in full_text.lower():
            return False

    return True


def can_dna_digivolve(evo_card: 'CardSource',
                      perm1: 'Permanent',
                      perm2: 'Permanent') -> bool:
    """Check if evo_card can DNA digivolve using perm1 and perm2.

    Tests both orderings: (perm1=req1, perm2=req2) and (perm1=req2, perm2=req1).
    Both permanents must be distinct.

    Delegates to get_dna_stacking_order() for the actual matching logic.
    """
    if not evo_card.is_digimon:
        return False
    if perm1 is perm2:
        return False
    return get_dna_stacking_order(evo_card, perm1, perm2) is not None


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
