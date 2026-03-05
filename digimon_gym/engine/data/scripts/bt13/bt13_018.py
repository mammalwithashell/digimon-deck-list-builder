from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_018(CardScript):
    """BT13-018 ShineGreymon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-018 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "RizeGreymon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Until the end of your opponent's turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon, can't digivolve, and gains <Blocker>.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT13-018 Your 1 [Marcus Damon] becomes Digimon")
        effect1.set_effect_description("[Start of Your Main Phase] Until the end of your opponent's turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon, can't digivolve, and gains <Blocker>.")
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_blocker')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until the end of your opponent's turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon, can't digivolve, and gains <Blocker>.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT13-018 Your 1 [Marcus Damon] becomes Digimon")
        effect2.set_effect_description("[When Digivolving] Until the end of your opponent's turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon, can't digivolve, and gains <Blocker>.")
        effect2.is_when_digivolving = True
        effect2._is_blocker = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_blocker')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns][Once Per Turn] When one of your red or yellow Tamers becomes suspended, 1 of your opponent's Digimon gets -6000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT13-018 DP -6000")
        effect3.set_effect_description("[All Turns][Once Per Turn] When one of your red or yellow Tamers becomes suspended, 1 of your opponent's Digimon gets -6000 DP for the turn.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("DP-6000_BT13_018")
        effect3.dp_modifier = -6000

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -6000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-6000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
