from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_090(CardScript):
    """Auto-transpiled from DCGO BT14_090.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-090 Ignore color requirements")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check name: "Tai Kamiya" in card name
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] By placing 1 [Greymon] and 1 [MetalGreymon] from your trash as 1 of your [Agumon]'s bottom digivolution cards, that Digimon may digivolve into [WarGreymon] in your hand without paying the cost, ignoring its digivolution requirements.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-090 Digivolve")
        effect1.set_effect_description("[Main] By placing 1 [Greymon] and 1 [MetalGreymon] from your trash as 1 of your [Agumon]'s bottom digivolution cards, that Digimon may digivolve into [WarGreymon] in your hand without paying the cost, ignoring its digivolution requirements.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Digivolve"""
            # digivolve_into(target_card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-090 Play Card, Trash From Hand, Add To Hand")
        effect2.set_effect_description("[Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Play Card, Trash From Hand, Add To Hand"""
            # play_card_from_hand_or_trash()
            # card.owner.trash_from_hand(count)
            # add_card_to_hand()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
