from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_019(CardScript):
    """Auto-transpiled from DCGO BT14_019.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, trash its bottom 2 digivolution cards.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-019 Trash 2 digivolution cards")
        effect0.set_effect_description("[Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, trash its bottom 2 digivolution cards.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("TrashDigivolution_BT14_019")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Trash Digivolution Cards"""
            # target.trash_digivolution_cards(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
