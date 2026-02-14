from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_078(CardScript):
    """BT23-078"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # Gain 1 memory (Tamer)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-078 Gain 1 memory (Tamer)")
        effect0.set_effect_description("Gain 1 memory (Tamer)")
        # [Start of Main] Gain 1 memory if opponent has Digimon

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When your Digimon are played or digivolve, if any of them have [Avian], [Bird], [Beast], [Animal] or [Sovereign] in any of their traits (other than [Sea Animal]) or the [CS] trait, by returning this Tamer to the hand, 1 of your Digimon gets +3000 DP for the turn. Then, 1 of your Digimon may attack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-078 By returning tamer to hand, 1 digimon gains +3K DP. then 1 digimon may attack")
        effect1.set_effect_description("[Your Turn] When your Digimon are played or digivolve, if any of them have [Avian], [Bird], [Beast], [Animal] or [Sovereign] in any of their traits (other than [Sea Animal]) or the [CS] trait, by returning this Tamer to the hand, 1 of your Digimon gets +3000 DP for the turn. Then, 1 of your Digimon may attack.")
        effect1.is_optional = True
        effect1.is_on_play = True
        effect1.dp_modifier = 3000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +3000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-078 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
