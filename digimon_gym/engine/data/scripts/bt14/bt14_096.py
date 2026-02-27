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
        # [Main] Suspend 1 of your opponent's Digimon. Then, if you have a Tamer with [Mimi Tachikawa] in its name,
        # 1 of your opponent's Digimon doesn't unsuspend until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-096 Suspend, Gain Keyword Cannot Unsuspend")
        effect0.set_effect_description("[Main] Suspend 1 of your opponent's Digimon. Then, if you have a Tamer with [Mimi Tachikawa] in its name, 1 of your opponent's Digimon doesn't unsuspend until the end of their turn.")
        effect0._is_cannot_unsuspend = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend, then conditionally apply cannot-unsuspend to an opponent Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def opponent_digimon_filter(p):
                return getattr(p, 'is_digimon', lambda: False)()

            # Suspend 1 of opponent's Digimon
            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=opponent_digimon_filter, is_optional=False)

            # If you have a Tamer with [Mimi Tachikawa] in name, choose 1 opponent Digimon
            # that doesn't unsuspend until end of their turn.
            has_mimi_tamer = False
            for p in player.get_permanents():
                if getattr(p, 'is_tamer', lambda: False)() and p.contains_card_name('Mimi Tachikawa'):
                    has_mimi_tamer = True
                    break

            if has_mimi_tamer:
                def on_cannot_unsuspend(target_perm):
                    target_perm.grant_keyword('_is_cannot_unsuspend')

                game.effect_select_opponent_permanent(
                    player, on_cannot_unsuspend, filter_fn=opponent_digimon_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
