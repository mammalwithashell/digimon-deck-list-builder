from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_095(CardScript):
    """BT24-095 Sonic Shot"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-095 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # Security: Activate this card's [Main] effects.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT24-095 Security: Activate this card's [Main] effects")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            effect2.on_process_callback(ctx)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # Suspend, Gain Keyword Cannot Unsuspend
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("BT24-095 Suspend 1 of opponent's Digimon or Tamers. It can't unsuspend in their next unsuspend phase. Then, you may link this card.")
        effect2.set_effect_description("Suspend, Gain Keyword Cannot Unsuspend")
        effect2._is_cannot_unsuspend = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon or p.is_tamer

            def on_suspend(target_perm):
                target_perm.suspend()
                # Match the engine's turn-based duration convention so the
                # keyword lasts through the target controller's next unsuspend
                # phase regardless of whose turn this resolves on.
                duration = game.turn_count + (2 if player.enemy is game.turn_player else 1)
                target_perm.grant_keyword('_is_cannot_unsuspend', duration=duration)

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            game.effect_link_to_permanent(player, card, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
