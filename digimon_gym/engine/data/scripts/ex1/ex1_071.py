from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_071(CardScript):
    """EX1-071 Win Rate: 60%!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 0: While you have a Tamer, ignore color requirements
        effect0 = ICardEffect()
        effect0.set_effect_name("EX1-071 Ignore color requirements")
        effect0.set_effect_description("While you have a Tamer, you can ignore this card's color requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore color requirements"""
            # descriptive-tagged: color requirement bypass not modeled
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Effect 1: OptionSkill — reduce next digivolve cost by 4
        # NOTE: Conditional future cost reduction with trash-from-hand-as-cost
        # is not fully modelable. Marked PARTIAL.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("EX1-071 Reduce next evo cost by 4")
        effect1.set_effect_description("[Main] The next time one of your Digimon would digivolve this turn, you may trash 1 Digimon card in your hand of the same color as the digivolving Digimon to reduce the memory cost of the digivolution by 4.")
        effect1.cost_reduction = 4

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reduce next evo cost by 4"""
            # descriptive-tagged: conditional future cost reduction with
            # trash-from-hand-as-cost not implementable in current engine
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Security Effect: Add this card to the hand.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX1-071 Add to hand")
        effect2.set_effect_description("[Security] Add this card to the hand.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Add to hand"""
            player = ctx.get('player')
            if player and card:
                player.hand_cards.append(card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
