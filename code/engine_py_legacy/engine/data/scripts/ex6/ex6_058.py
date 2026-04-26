from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_058(CardScript):
    """EX6-058 Creepymon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-058 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with the lowest DP. For each level of the Digimon deleted by this effect, trash the top card of your deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX6-058 Delete 1 opponent Digimon with the lowest DP")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with the lowest DP. For each level of the Digimon deleted by this effect, trash the top card of your deck.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with the lowest DP. For each level of the Digimon deleted by this effect, trash the top card of your deck.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX6-058 Delete 1 opponent Digimon with the lowest DP")
        effect2.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with the lowest DP. For each level of the Digimon deleted by this effect, trash the top card of your deck.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area other than in battle, place 1 card with the [Seven Great Demon Lord] trait from your trash as the bottom digivolution cards of one of your [Gate of Deadly Sins] in your breeding area.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("EX6-058 Place 1 7GDL from trash to bottom of [Gate of Deadly Sins]")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area other than in battle, place 1 card with the [Seven Great Demon Lord] trait from your trash as the bottom digivolution cards of one of your [Gate of Deadly Sins] in your breeding area.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
