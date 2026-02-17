from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_012(CardScript):
    """P-012 Tai Kamiya (V-Tamer)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Main] If you have a Digimon with [Veedramon] in its name, you may suspend this Tamer to activate one of the following effects: - Trigger <Draw 1>. (Draw 1 card from your deck. - 1 of your Digimon gets +1000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-012 Draw 1 or your 1 Digimon gains DP +1000")
        effect0.set_effect_description("[Main] If you have a Digimon with [Veedramon] in its name, you may suspend this Tamer to activate one of the following effects: - Trigger <Draw 1>. (Draw 1 card from your deck. - 1 of your Digimon gets +1000 DP for the turn.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, DP +1000, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if perm:
                perm.change_dp(1000)
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Veedramon')):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("P-012 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
