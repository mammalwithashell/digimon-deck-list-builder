from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_086(CardScript):
    """BT21-086 Marcus Damon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-086 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT21-086 Memory +1")
        effect1.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Condition: opponent must have at least 1 Digimon
            player = card.owner
            if not any(p.is_digimon for p in player.enemy.battle_area):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your [Marcus Damon]s may suspend.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-086 Suspend a Marcus Damon")
        effect2.set_effect_description("[On Play] 1 of your [Marcus Damon]s may suspend.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend 1 of your Marcus Damon tamers"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_suspend(target):
                target.suspend()
            game.effect_select_own_permanent(
                player, on_suspend,
                filter_fn=lambda p: p.contains_card_name('Marcus Damon'),
                is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns][Once Per Turn] When this Tamer suspends, 1 of your Digimon gains <Piercing> and +3000 DP for the turn. Then, 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT21-086 Piercing +3000 DP to ally, -3000 DP to enemy")
        effect3.set_effect_description("[All Turns][Once Per Turn] When this Tamer suspends, 1 of your Digimon gains <Piercing> and +3000 DP for the turn. Then, 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("PiercingAndDPMinus_BT21_086")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # "When THIS Tamer suspends" — the suspended permanent must be this Marcus Damon
            suspended_perm = context.get('permanent')
            this_perm = card.permanent_of_this_card()
            if suspended_perm is not this_perm:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Select own Digimon for Piercing +3000 DP, then select opponent Digimon for -3000 DP"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: 1 of your Digimon gains Piercing and +3000 DP
            def on_ally_buff(target):
                target.grant_keyword('_is_piercing')
                target.change_dp(3000)

            # Step 2: Then, 1 of your opponent's Digimon gets -3000 DP
            def on_enemy_debuff(target):
                target.change_dp(-3000)

            game.effect_select_own_permanent(
                player, on_ally_buff,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False)
            game.effect_select_opponent_permanent(
                player, on_enemy_debuff,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
