from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_093(CardScript):
    """Auto-transpiled from DCGO BT14_093.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Search your security stack. 1 of your Digimon may digivolve into 1 yellow level 6 or lower Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, and you have a Tamer with [T.K. Takaishi] in its name, <Recovery +1 (Deck)>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-093 Recovery +1, Play Card")
        effect0.set_effect_description("[Main] Search your security stack. 1 of your Digimon may digivolve into 1 yellow level 6 or lower Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, and you have a Tamer with [T.K. Takaishi] in its name, <Recovery +1 (Deck)>.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check trait: "Vaccine" in target traits
            # Check name: "T.K. Takaishi" in card name
            # Check color: CardColor.Yellow
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Recovery +1, Play Card"""
            # card.owner.recover(1)
            # play_card_from_hand_or_trash()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Patamon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-093 Play Card, Trash From Hand, Add To Hand")
        effect1.set_effect_description("[Security] You may play 1 [Patamon] from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Play Card, Trash From Hand, Add To Hand"""
            # play_card_from_hand_or_trash()
            # card.owner.trash_from_hand(count)
            # add_card_to_hand()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
