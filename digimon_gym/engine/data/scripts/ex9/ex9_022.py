from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_022(CardScript):
    """EX9-022 Elecmon | Lv.3 | Mammal/DM/Ver.2
    <Training>
    Inherited: [Your Turn] All of your opponent's Security Digimon get -3000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-022 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Inherited: [Your Turn] All opponent Security Digimon get -3000 DP
        # This is a continuous modifier during your turn
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.NoTiming)
        effect1.set_effect_name("EX9-022 Inherited: Opponent Security Digimon -3000 DP")
        effect1.set_effect_description(
            "Inherited: [Your Turn] All opponent Security Digimon get -3000 DP."
        )
        effect1.is_inherited_effect = True
        effect1._security_digimon_dp_modifier = -3000

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
