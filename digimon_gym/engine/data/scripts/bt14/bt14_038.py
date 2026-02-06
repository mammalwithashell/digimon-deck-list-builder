from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_038(CardScript):
    """Auto-transpiled from DCGO BT14_038.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.SecuritySkill
        # [Security] If you have 3 or more cards with [Sukamon] in their names in your trash, you may play 1 level 6 Digimon card with [Etemon] in its name from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-038 Play 1 Digimon from hand")
        effect0.set_effect_description("[Security] If you have 3 or more cards with [Sukamon] in their names in your trash, you may play 1 level 6 Digimon card with [Etemon] in its name from your hand without paying the cost.")
        effect0.is_optional = True
        effect0.is_security_effect = True
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check name: "Etemon" in card name
            # Check name: "Sukamon" in card name
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Play Card, Trash From Hand"""
            # play_card_from_hand_or_trash()
            # card.owner.trash_from_hand(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place this card at the bottom of your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-038 Place this card at the bottom of security")
        effect1.set_effect_description("[On Deletion] Place this card at the bottom of your security stack.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Add To Security"""
            # card.owner.add_to_security()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 [Etemon] from your trash at the bottom of your security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-038 Place 1 [Etemon] from trash at the bottom of security")
        effect2.set_effect_description("[On Deletion] Place 1 [Etemon] from your trash at the bottom of your security stack.")
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Add To Security"""
            # card.owner.add_to_security()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
