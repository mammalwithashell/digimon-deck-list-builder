"""Digivolution validation logic.

Provides can_digivolve() which checks whether a card from hand can legally
digivolve onto a permanent on the field, using the card's evo_costs data.
"""

from __future__ import annotations
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..core.card_source import CardSource
    from ..core.permanent import Permanent


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
