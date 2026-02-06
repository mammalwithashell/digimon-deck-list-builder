from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_076(CardScript):
    """Auto-transpiled from DCGO BT14_076.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 1 card in your hand, delete 1 of your Digimon with the lowest level and 1 of your opponent's Digimon with the lowest level.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-076 Trash 1 card from hand to delete Digimons")
        effect0.set_effect_description("[When Digivolving] By trashing 1 card in your hand, delete 1 of your Digimon with the lowest level and 1 of your opponent's Digimon with the lowest level.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Delete, Trash From Hand"""
            # target_permanent.delete()
            # card.owner.trash_from_hand(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 [Agumon] from your trash without paying the cost. If you have a Tamer with [Tai Kamiya] in its name, that Digimon gains <Rush> for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-076 Play 1 [Agumon] from trash")
        effect1.set_effect_description("[On Deletion] You may play 1 [Agumon] from your trash without paying the cost. If you have a Tamer with [Tai Kamiya] in its name, that Digimon gains <Rush> for the turn.")
        effect1.is_optional = True
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check name: "Tai Kamiya" in card name
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Play Card"""
            # play_card_from_hand_or_trash()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
