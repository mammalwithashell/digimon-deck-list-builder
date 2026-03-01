from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_103(CardScript):
    """BT3-103 Hidden Potential Discovered!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] For the turn, when one of your green Digimon would next
        # digivolve, by suspending 1 of your Digimon, reduce the digivolution
        # cost by 5.
        # NOTE: Conditional future cost reduction with suspend-as-cost is not
        # fully modelable. We register a cost_reduction of 5 but cannot enforce
        # the suspend requirement. Marked PARTIAL.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT3-103 Reduce next green evo cost by 5")
        effect0.set_effect_description("[Main] For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5.")
        effect0.cost_reduction = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reduce next green evo cost by 5"""
            # descriptive-tagged: conditional future cost reduction with
            # suspend-as-cost not implementable in current engine
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security Effect: Add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT3-103 Add to hand")
        effect1.set_effect_description("[Security] Add this card to the hand.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add to hand"""
            player = ctx.get('player')
            if player and card:
                player.hand_cards.append(card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
