from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_003(CardScript):
    """BT23-003"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When any of your [CS] trait Option cards are placed in the battle area, this Digimon may attack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-003 This digimon may attack")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When any of your [CS] trait Option cards are placed in the battle area, this Digimon may attack.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT23_003_YT")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
