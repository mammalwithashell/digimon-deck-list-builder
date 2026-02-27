from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_024(CardScript):
    """EX5-024 Azulongmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-024 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-024 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return 1 of your opponent's level 5 or lower Digimon to the hand. Then, unsuspend 1 of your Digimon with the [Deva]/[Four Great Dragons]/[Four Sovereigns] trait.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-024 Return 1 level 5 or lower Digimon to hand and unsuspend 1 Digimon")
        effect2.set_effect_description("[On Play] Return 1 of your opponent's level 5 or lower Digimon to the hand. Then, unsuspend 1 of your Digimon with the [Deva]/[Four Great Dragons]/[Four Sovereigns] trait.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Bounce, Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                if not (any('Deva' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Four Sovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('FourSovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Four Great Dragons' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('FourGreatDragons' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return 1 of your opponent's level 5 or lower Digimon to the hand. Then, unsuspend 1 of your Digimon with the [Deva]/[Four Great Dragons]/[Four Sovereigns] trait.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-024 Return 1 level 5 or lower Digimon to hand and unsuspend 1 Digimon")
        effect3.set_effect_description("[When Digivolving] Return 1 of your opponent's level 5 or lower Digimon to the hand. Then, unsuspend 1 of your Digimon with the [Deva]/[Four Great Dragons]/[Four Sovereigns] trait.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Bounce, Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                if not (any('Deva' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Four Sovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('FourSovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Four Great Dragons' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('FourGreatDragons' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Delete 1 of your opponent's Digimon with the highest level.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX5-024 Delete opponent's all Digimons with the highest level")
        effect4.set_effect_description("[On Deletion] Delete 1 of your opponent's Digimon with the highest level.")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
