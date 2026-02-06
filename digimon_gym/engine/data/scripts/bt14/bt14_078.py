from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_078(CardScript):
    """Auto-transpiled from DCGO BT14_078.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] Delete this Digimon and <Draw 2>. Then, you may return 1 [Loogamon] from your trash to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-078 Delete this Digimon, Draw 2 and return 1 card from trash to hand")
        effect0.set_effect_description("[End of Your Turn] Delete this Digimon and <Draw 2>. Then, you may return 1 [Loogamon] from your trash to the hand.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Draw 2, Delete, Add To Hand"""
            # card.owner.draw(2)
            # target_permanent.delete()
            # add_card_to_hand()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may trash up to 3 cards with the [Dark Animal] or [SoC] trait in your hand. Then, delete 1 of your opponent's level 3 or lower Digimon. For each card trashed by this effect, add 1 to the level this effect may choose.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-078 Trash cards from hand and delete 1 Digimon")
        effect1.set_effect_description("[On Deletion] You may trash up to 3 cards with the [Dark Animal] or [SoC] trait in your hand. Then, delete 1 of your opponent's level 3 or lower Digimon. For each card trashed by this effect, add 1 to the level this effect may choose.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check trait: "Dark Animal" in target traits
            # Check trait: "DarkAnimal" in target traits
            # Check trait: "SoC" in target traits
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Delete, Trash From Hand"""
            # target_permanent.delete()
            # card.owner.trash_from_hand(count)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
