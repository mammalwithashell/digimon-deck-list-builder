from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_082(CardScript):
    """Auto-transpiled from DCGO BT14_082.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] 1 of your Digimon with the [Vaccine] trait gets +2000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-082 DP +2000")
        effect0.set_effect_description("[Start of Your Main Phase] 1 of your Digimon with the [Vaccine] trait gets +2000 DP for the turn.")
        effect0.dp_modifier = 2000

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check trait: "Vaccine" in target traits
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: DP +2000"""
            # target.change_dp(2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # [Your Turn] When a card is removed from your opponent's security stack, by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-082 Memory +1")
        effect1.set_effect_description("[Your Turn] When a card is removed from your opponent's security stack, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Gain 1 memory, Suspend"""
            # card.owner.add_memory(1)
            # target_permanent.suspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-082 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True
        # Security effect: play this card without paying cost
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
