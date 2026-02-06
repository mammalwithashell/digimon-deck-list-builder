from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_102(CardScript):
    """Auto-transpiled from DCGO BT14_102.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By deleting this Digimon, activate 1 of the effects below: - Place 1 of your opponent's Digimon with the [Virus] trait at the bottom of their security stack. - 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-102 Delete this Digimon to select effects")
        effect0.set_effect_description("[When Attacking] By deleting this Digimon, activate 1 of the effects below: - Place 1 of your opponent's Digimon with the [Virus] trait at the bottom of their security stack. - 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect0.is_optional = True
        effect0.is_on_attack = True
        effect0.dp_modifier = -5000

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check trait: "Virus" in target traits
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: DP -5000"""
            # target.change_dp(-5000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place this card at the bottom of your security stack. Then, if you have a Tamer, you may hatch in your breeding area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-102 Place this card at the bottom of security and hatch")
        effect1.set_effect_description("[On Deletion] Place this card at the bottom of your security stack. Then, if you have a Tamer, you may hatch in your breeding area.")
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
        # [On Deletion] Place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack. 
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-102 Place 1 card from hand at the bottom security")
        effect2.set_effect_description("[On Deletion] Place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack. ")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check trait: "Vaccine" in target traits
            # Check color: CardColor.Yellow
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Trash From Hand, Add To Security"""
            # card.owner.trash_from_hand(count)
            # card.owner.add_to_security()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
