from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_021(CardScript):
    """BT22-021 Shellmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: decode
        # Decode
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-021 Decode")
        effect0.set_effect_description("Decode")
        effect0._is_decode = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 level 5 or lower Digimon card with [Aqua] or [Sea Animal] in any of its traits from your hand as any of your Digimon's bottom digivolution card.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-021 Place 1 level 5 or lower [Aqua]/[Sea Animal] digimon from hand under any digimon")
        effect1.set_effect_description("[On Play] You may place 1 level 5 or lower Digimon card with [Aqua] or [Sea Animal] in any of its traits from your hand as any of your Digimon's bottom digivolution card.")
        effect1.is_optional = True
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
        # [When Digivolging] You may place 1 level 5 or lower Digimon card with [Aqua] or [Sea Animal] in any of its traits from your hand as any of your Digimon's bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-021 Place 1 level 5 or lower [Aqua]/[Sea Animal] digimon from hand under any digimon")
        effect2.set_effect_description("[When Digivolging] You may place 1 level 5 or lower Digimon card with [Aqua] or [Sea Animal] in any of its traits from your hand as any of your Digimon's bottom digivolution card.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: jamming
        # Jamming
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-021 Jamming")
        effect3.set_effect_description("Jamming")
        effect3.is_inherited_effect = True
        effect3._is_jamming = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
