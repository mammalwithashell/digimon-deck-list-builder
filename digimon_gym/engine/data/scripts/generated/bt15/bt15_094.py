from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_094(CardScript):
    """BT15-094 Super Shocker"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Suspend 1 level 6 or lower Digimon. 1 of your Digimon with the [Insectoid] trait gets +3000 DP until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-094 DP +3000, Suspend")
        effect0.set_effect_description("[Main] Suspend 1 level 6 or lower Digimon. 1 of your Digimon with the [Insectoid] trait gets +3000 DP until the end of your opponent's turn.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +3000, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 6:
                    return False
                if not (any('Insectoid' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
