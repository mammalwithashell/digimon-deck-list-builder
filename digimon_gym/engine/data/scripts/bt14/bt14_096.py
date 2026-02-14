from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_096(CardScript):
    """BT14-096 Blooming of Sincerity"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Suspend 1 of your opponent's Digimon. Then, if you have a Tamer with [Mimi Tachikawa] in its name, 1 of your opponent's Digimon doesn't unsuspend until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-096 Suspend")
        effect0.set_effect_description("[Main] Suspend 1 of your opponent's Digimon. Then, if you have a Tamer with [Mimi Tachikawa] in its name, 1 of your opponent's Digimon doesn't unsuspend until the end of their turn.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Mimi Tachikawa')):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
