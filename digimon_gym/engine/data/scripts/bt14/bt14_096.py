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
            """Action: Suspend opponent Digimon; optionally apply cannot-unsuspend to opponent Digimon if Mimi is present."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Always suspend 1 of opponent's Digimon.
            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_digimon(
                player, on_suspend, is_optional=False)

            # Then check your Tamers for Mimi Tachikawa and apply cannot-unsuspend to 1 opponent Digimon.
            has_mimi_tamer = False
            for p in player.get_permanents():
                if getattr(p, 'is_tamer', lambda: False)() and p.contains_card_name('Mimi Tachikawa'):
                    has_mimi_tamer = True
                    break

            if has_mimi_tamer:
                def on_cannot_unsuspend(target_perm):
                    target_perm.grant_keyword('_is_cannot_unsuspend')

                game.effect_select_opponent_digimon(
                    player, on_cannot_unsuspend, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
