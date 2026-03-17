from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_014(CardScript):
    """BT20-014 SaviorHuckmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with 5000DP or less.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT20-014 Delete a Digimon with 5000DP or less")
        effect0.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with 5000DP or less.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 5000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with 5000DP or less.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT20-014 Delete a Digimon with 5000DP or less")
        effect1.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with 5000DP or less.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 5000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] By suspending 1 of your other Digimon, this Digimon may digivolve into a Digimon card with [Jesmon] in its name in the hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT20-014 By suspending a Digimon, this Digimon may digivolve into a Digimon card with [Jesmon] in its name")
        effect2.set_effect_description("[End of Your Turn] By suspending 1 of your other Digimon, this Digimon may digivolve into a Digimon card with [Jesmon] in its name in the hand without paying the cost.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: By suspending 1 of your other Digimon, digivolve into [Jesmon] from hand free."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # "By suspending 1 of your other Digimon" = COST (happens first)
            def suspend_filter(p):
                return p is not perm and p.is_digimon and not p.is_suspended
            def on_suspend(target_perm):
                target_perm.suspend()
                # THEN digivolve this Digimon into [Jesmon] from hand without paying cost
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    return any('Jesmon' in _n for _n in getattr(c, 'card_names', []))
                game.effect_digivolve_from_hand(
                    player, perm, digi_filter, cost_override=0, is_optional=True)
            game.effect_select_own_permanent(
                player, on_suspend, filter_fn=suspend_filter, is_optional=True,
                prompt="Suspend 1 of your other Digimon to digivolve into [Jesmon].")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: alliance
        # Alliance
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-014 Alliance")
        effect3.set_effect_description("Alliance")
        effect3.is_inherited_effect = True
        effect3.is_on_attack = True
        effect3._is_alliance = True

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
