from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_072(CardScript):
    """BT15-072 Vilemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-072 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When one of your [Apocalymon] or Digimon with the [Dark Masters] trait would leave the battle area other than by one of your effects, by deleting this Digimon, prevent 1 of those Digimon from leaving play.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-072 Delete this Digimon to prevent 1 other Digimon from leaving Battle Area")
        effect1.set_effect_description("[All Turns] When one of your [Apocalymon] or Digimon with the [Dark Masters] trait would leave the battle area other than by one of your effects, by deleting this Digimon, prevent 1 of those Digimon from leaving play.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Substitute_BT15_072")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
