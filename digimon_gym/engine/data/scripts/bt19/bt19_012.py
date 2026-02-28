from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_012(CardScript):
    """BT19-012 OmniShoutmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-012 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Xros Heart] trait for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_trait = "Xros Heart"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-012 Also treated as [Shoutmon] for a DigiXros")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, delete 1 of your opponent's Digimon with 3000 DP or less.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-012 -3000 DP for the turn and delete 3000 DP or less Digimon")
        effect2.set_effect_description("[On Play] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, delete 1 of your opponent's Digimon with 3000 DP or less.")
        effect2.set_max_count_per_turn(1)
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -3000, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 3000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, delete 1 of your opponent's Digimon with 3000 DP or less.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-012 -3000 DP for the turn and delete 3000 DP or less Digimon")
        effect3.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, delete 1 of your opponent's Digimon with 3000 DP or less.")
        effect3.set_max_count_per_turn(1)
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -3000, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 3000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 Digimon card with the [Xros Heart]/[Blue Flare] trait from your hand or trash under your Tamers.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT19-012 Place 1 [Xros Heart]/[Blue Flare] card from trash under 1 of your Tamers, then <Save>")
        effect4.set_effect_description("[On Deletion] Place 1 Digimon card with the [Xros Heart]/[Blue Flare] trait from your hand or trash under your Tamers.")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: rush
        # Rush
        effect5 = ICardEffect()
        effect5.set_effect_name("BT19-012 Rush")
        effect5.set_effect_description("Rush")
        effect5.is_inherited_effect = True
        effect5._is_rush = True

        def condition5(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
