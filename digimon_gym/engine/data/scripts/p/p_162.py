from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_162(CardScript):
    """P-162 Coelamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-162 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 with [DS] trait for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "DS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('DS' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Until the end of your opponent's turn, their effects can't reduce the DP of 1 of your [DS] trait Digimon or affect it with <De-Digivolve> effects.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-162 Opponent's effects can't reduce a DS digimon's DP or de-digivolve it")
        effect1.set_effect_description("[On Play] Until the end of your opponent's turn, their effects can't reduce the DP of 1 of your [DS] trait Digimon or affect it with <De-Digivolve> effects.")
        effect1.is_on_play = True
        effect1._is_immune_dp_minus = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Immune Dp Minus"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_immune_dp_minus')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until the end of your opponent's turn, their effects can't reduce the DP of 1 of your [DS] trait Digimon or affect it with <De-Digivolve> effects.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-162 Opponent's effects can't reduce a DS digimon's DP or de-digivolve it")
        effect2.set_effect_description("[When Digivolving] Until the end of your opponent's turn, their effects can't reduce the DP of 1 of your [DS] trait Digimon or affect it with <De-Digivolve> effects.")
        effect2.is_when_digivolving = True
        effect2._is_immune_dp_minus = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Immune Dp Minus"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_immune_dp_minus')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("P-162 Blocker")
        effect3.set_effect_description("Blocker")
        effect3.is_inherited_effect = True
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
