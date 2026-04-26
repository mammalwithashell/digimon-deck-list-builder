from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_175(CardScript):
    """P-175 Hina Kurihara"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartTurn
        # [Your Turn] When any of your Digimon with the [Rock Dragon] or [Machine Dragon] trait are played, by suspending this Tamer, 1 of your level 4 or higher Digimon may digivolve into a Digimon card with the [Rock Dragon], [Earth Dragon], [Machine Dragon] or [Sky Dragon] trait in the hand with the digivolution cost reduced by 2.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("P-175 Digivolve, for reduced cost of 2")
        effect0.set_effect_description("[Your Turn] When any of your Digimon with the [Rock Dragon] or [Machine Dragon] trait are played, by suspending this Tamer, 1 of your level 4 or higher Digimon may digivolve into a Digimon card with the [Rock Dragon], [Earth Dragon], [Machine Dragon] or [Sky Dragon] trait in the hand with the digivolution cost reduced by 2.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level < 4:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level < 4:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your Digimon with the [Rock Dragon] or [Machine Dragon] trait are played, by suspending this Tamer, 1 of your level 4 or higher Digimon may digivolve into a Digimon card with the [Rock Dragon], [Earth Dragon], [Machine Dragon] or [Sky Dragon] trait in the hand with the digivolution cost reduced by 2.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-175 Digivolve, for reduced cost of 2")
        effect1.set_effect_description("[Your Turn] When any of your Digimon with the [Rock Dragon] or [Machine Dragon] trait are played, by suspending this Tamer, 1 of your level 4 or higher Digimon may digivolve into a Digimon card with the [Rock Dragon], [Earth Dragon], [Machine Dragon] or [Sky Dragon] trait in the hand with the digivolution cost reduced by 2.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level < 4:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level < 4:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
