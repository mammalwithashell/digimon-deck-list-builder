from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_080(CardScript):
    """BT11-080 Devimon | Lv.4 Purple Fallen Angel | DP 5000 | Cost 5

    [Your Turn] While you have a yellow Digimon or yellow Tamer in play,
    this Digimon gains <Rush> (This Digimon can attack the turn it comes
    into play.) and <Retaliation> (When this Digimon is deleted after losing
    a battle, delete the Digimon it was battling.)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _conditional_active(context: Dict[str, Any]) -> bool:
            """[Your Turn] + yellow Digimon/Tamer in play + self on field."""
            if not card:
                return False
            # Field presence: must be on battle area
            if card.permanent_of_this_card() is None:
                return False
            owner = card.owner
            if not owner:
                return False
            # [Your Turn] gate
            if not owner.is_my_turn:
                return False
            # "you have a yellow Digimon or yellow Tamer in play"
            for perm in owner.battle_area:
                top = getattr(perm, 'top_card', None)
                if top is None:
                    continue
                if not (perm.is_digimon or perm.is_tamer):
                    continue
                colors = getattr(top, 'card_colors', None) or []
                if CardColor.Yellow in colors:
                    return True
            return False

        # Factory effect: rush
        # Rush — conditionally granted
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-080 Rush")
        effect0.set_effect_description(
            "[Your Turn] While you have a yellow Digimon or yellow Tamer in "
            "play, this Digimon gains <Rush>."
        )
        effect0._is_rush = True
        effect0.set_can_use_condition(_conditional_active)
        effects.append(effect0)

        # Factory effect: retaliation
        # Retaliation — conditionally granted
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-080 Retaliation")
        effect1.set_effect_description(
            "[Your Turn] While you have a yellow Digimon or yellow Tamer in "
            "play, this Digimon gains <Retaliation>."
        )
        effect1.is_on_deletion = True
        effect1._is_retaliation = True
        effect1.set_can_use_condition(_conditional_active)
        effects.append(effect1)

        return effects
