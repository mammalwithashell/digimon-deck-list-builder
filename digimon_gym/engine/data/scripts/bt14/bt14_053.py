from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_053(CardScript):
    """Auto-transpiled from DCGO BT14_053.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-053 Suspend 1 Digimon or Tamer")
        effect0.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Suspend 1 of your opponent's Digimon or Tamers.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-053 Suspend 1 Digimon or Tamer")
        effect1.set_effect_description("[When Attacking] Suspend 1 of your opponent's Digimon or Tamers.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When an effect suspends a Digimon or Tamer, you may unsuspend this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-053 Unsuspend this Digimon")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When an effect suspends a Digimon or Tamer, you may unsuspend this Digimon.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Unsuspend_BT14_053")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
