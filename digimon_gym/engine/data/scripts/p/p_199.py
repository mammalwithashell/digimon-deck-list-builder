from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_199(CardScript):
    """P-199 Dan Yuki"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If you have 4 or less memory, 1 of your Digimon gets +3000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-199 +3K DP")
        effect0.set_effect_description("[Start of Your Main Phase] If you have 4 or less memory, 1 of your Digimon gets +3000 DP for the turn.")
        effect0.dp_modifier = 3000

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +3000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Cost -1
        effect1 = ICardEffect()
        effect1.set_effect_name("P-199 Play Cost -1")
        effect1.set_effect_description("Cost -1")
        effect1.cost_reduction = 1

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("P-199 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
