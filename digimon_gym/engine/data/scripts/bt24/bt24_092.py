from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_092(CardScript):
    """BT24-092 Shock Plasma"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-092 Ignore color requirements")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-092 1 opponent's Digimon gets -6K DP for the turn. Then, you may link this card.")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -6000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-092 1 Opponent's digimon gets -6K DP for the turn.")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -6000 DP for the turn.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("WA_BT24-092")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
