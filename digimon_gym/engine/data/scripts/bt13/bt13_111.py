from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_111(CardScript):
    """BT13-111 Gallantmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-111 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-111 Delete 1 Digimon with 6000 DP or less, or delete 1 Digimon with 13000 DP or more")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
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
                if p.dp is None or p.dp > 6000:
                    return False
                if p.dp is None or p.dp < 13000:
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-111 Delete 1 Digimon with 6000 DP or less, or delete 1 Digimon with 13000 DP or more")
        effect2.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 6000:
                    return False
                if p.dp is None or p.dp < 13000:
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

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-111 Delete 1 Digimon with 6000 DP or less, or delete 1 Digimon with 13000 DP or more")
        effect3.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 6000:
                    return False
                if p.dp is None or p.dp < 13000:
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

        return effects
