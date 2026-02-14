from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_054(CardScript):
    """BT23-054"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-054 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Veemon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Veemon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Veemon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-054 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: armor_purge
        # Armor Purge
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-054 Armor Purge")
        effect2.set_effect_description("Armor Purge")
        effect2._is_armor_purge = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <Draw 1> Then, 1 of your Digimon with the [Royal Knight] or [CS] trait can't be returned to hands or decks by your opponent's effects until their turn ends.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-054 Draw 1, then give can't be returned to hand/deck")
        effect3.set_effect_description("[On Play] <Draw 1> Then, 1 of your Digimon with the [Royal Knight] or [CS] trait can't be returned to hands or decks by your opponent's effects until their turn ends.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <Draw 1> Then, 1 of your Digimon with the [Royal Knight] or [CS] trait can't be returned to hands or decks by your opponent's effects until their turn ends.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-054 Draw 1, then give can't be returned to hand/deck")
        effect4.set_effect_description("[When Digivolving] <Draw 1> Then, 1 of your Digimon with the [Royal Knight] or [CS] trait can't be returned to hands or decks by your opponent's effects until their turn ends.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
