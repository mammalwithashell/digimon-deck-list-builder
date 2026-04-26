"""DigiXros material validation for the RL action space.

Determines which cards from hand/field/trash can serve as DigiXros materials.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional, Set, Tuple

from ..data.evo_cost import DigiXrosCost, DigiXrosElement
from ..data.enums import CardColor
from ..game.constants import SEL_HAND_START, SEL_MY_FIELD_START, SEL_TRASH_START

if TYPE_CHECKING:
    from ..core.card_source import CardSource
    from ..core.permanent import Permanent
    from ..core.player import Player


def _card_matches_element(card: 'CardSource', element: DigiXrosElement,
                          xros_cost: DigiXrosCost) -> bool:
    """Check if a single card matches one DigiXros element condition."""
    # Digimon-only check
    if element.is_digimon_only and not card.is_digimon:
        return False

    # Color check
    if element.color is not None:
        if element.color not in card.card_colors:
            return False

    # Level check
    if element.level_max is not None:
        card_level = card.level
        if card_level is None or card_level > element.level_max:
            return False

    # has_text check (e.g., ＜Save＞ in text)
    if xros_cost.has_text:
        if xros_cost.has_text.lower() not in card.card_text.lower():
            return False

    # Name matching (with OR alternatives stored in trait_alternatives for name-OR patterns)
    if element.name_contains:
        if element.trait_match == '' and not element.trait_alternatives:
            # Pure name match (no trait involved)
            if not card.contains_card_name(element.name_contains):
                return False
        elif element.trait_match == '' and element.trait_alternatives:
            # OR-name pattern: name_contains has first name, trait_alternatives has rest
            names = [element.name_contains] + element.trait_alternatives
            if not any(card.contains_card_name(n) for n in names):
                return False
        else:
            # Trait match
            all_traits = [element.trait_match] + element.trait_alternatives
            if not any(
                trait.lower() in t.lower()
                for trait in all_traits
                for t in card.card_traits
            ):
                return False

    # Trait-only match (no name_contains, just trait_match)
    if not element.name_contains and element.trait_match:
        all_traits = [element.trait_match] + element.trait_alternatives
        if not any(
            trait.lower() in t.lower()
            for trait in all_traits
            for t in card.card_traits
        ):
            return False

    return True


def _matches_any_element(card: 'CardSource', xros_cost: DigiXrosCost) -> bool:
    """Check if a card matches at least one element in the DigiXros cost."""
    for element in xros_cost.elements:
        if _card_matches_element(card, element, xros_cost):
            return True
    return False


def _check_uniqueness(card: 'CardSource', xros_cost: DigiXrosCost,
                       already_selected: List['CardSource']) -> bool:
    """Check uniqueness constraints against already-selected materials."""
    if xros_cost.different_card_numbers:
        for sel in already_selected:
            if card.card_id == sel.card_id:
                return False
    if xros_cost.different_names:
        card_names_lower = {n.lower() for n in card.card_names}
        for sel in already_selected:
            sel_names_lower = {n.lower() for n in sel.card_names}
            if card_names_lower & sel_names_lower:
                return False
    return True


def _element_still_needs(element: DigiXrosElement, xros_cost: DigiXrosCost,
                          already_selected: List['CardSource']) -> int:
    """How many more cards this element can still accept."""
    count_matched = 0
    for sel in already_selected:
        if _card_matches_element(sel, element, xros_cost):
            count_matched += 1
    return max(0, element.count - count_matched)


def get_valid_digixros_materials(
    played_card: 'CardSource',
    player: 'Player',
    already_selected: List['CardSource'],
    xros_cost: Optional[DigiXrosCost] = None,
) -> List[int]:
    """Return action IDs for valid DigiXros material candidates.

    Checks hand, field, and trash based on xros_cost.source_zones.
    """
    if xros_cost is None:
        xros_cost = played_card.digixros_cost
    if xros_cost is None:
        return []

    # Check max materials
    if xros_cost.max_materials >= 0 and len(already_selected) >= xros_cost.max_materials:
        return []

    # Check if any element still needs materials
    total_remaining = sum(
        _element_still_needs(e, xros_cost, already_selected) for e in xros_cost.elements
    )
    if xros_cost.max_materials >= 0 and total_remaining <= 0:
        return []

    valid_ids: List[int] = []
    already_set = set(id(c) for c in already_selected)

    # Hand candidates
    if 'hand' in xros_cost.source_zones:
        for i, card in enumerate(player.hand_cards):
            if id(card) == id(played_card):
                continue
            if id(card) in already_set:
                continue
            if not _matches_any_element(card, xros_cost):
                continue
            if not _check_uniqueness(card, xros_cost, already_selected):
                continue
            valid_ids.append(SEL_HAND_START + i)

    # Field candidates
    if 'field' in xros_cost.source_zones:
        for i, perm in enumerate(player.battle_area):
            top = perm.top_card
            if top is None:
                continue
            if top.is_token:
                continue
            if id(top) in already_set:
                continue
            if not _matches_any_element(top, xros_cost):
                continue
            if not _check_uniqueness(top, xros_cost, already_selected):
                continue
            valid_ids.append(SEL_MY_FIELD_START + i)

    # Trash candidates
    if 'trash' in xros_cost.source_zones:
        for i, card in enumerate(player.trash_cards):
            if id(card) in already_set:
                continue
            if not _matches_any_element(card, xros_cost):
                continue
            if not _check_uniqueness(card, xros_cost, already_selected):
                continue
            valid_ids.append(SEL_TRASH_START + i)

    return valid_ids


def has_any_digixros_material(card: 'CardSource', player: 'Player') -> bool:
    """Quick check: does the player have at least one valid material for this card?"""
    return bool(get_valid_digixros_materials(card, player, []))
