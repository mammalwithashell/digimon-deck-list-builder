from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_069(CardScript):
    """BT23-069"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 level 5 or lower Digimon card with the [Ghost] trait from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-069 You may play 1 level 5 or lower Digimon")
        effect0.set_effect_description("[On Play] You may play 1 level 5 or lower Digimon card with the [Ghost] trait from your trash without paying the cost.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 5 or lower Digimon card with the [Ghost] trait from your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-069 You may play 1 level 5 or lower Digimon")
        effect1.set_effect_description("[On Deletion] You may play 1 level 5 or lower Digimon card with the [Ghost] trait from your trash without paying the cost.")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [All Turns] When another Digimon attacks, by deleting this Digimon, delete 1 of your opponent's level 6 or lower Digimon. If this effect didn't delete your opponent's Digimon, you may end that attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-069 By deleting this digimon, delete 1 level 6 or lower digimon. if you didnt delete, you may end the attack")
        effect2.set_effect_description("[All Turns] When another Digimon attacks, by deleting this Digimon, delete 1 of your opponent's level 6 or lower Digimon. If this effect didn't delete your opponent's Digimon, you may end that attack.")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
