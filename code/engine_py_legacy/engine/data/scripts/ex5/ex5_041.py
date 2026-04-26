from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_041(CardScript):
    """EX5-041 Ebonwumon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-041 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Deva"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-041 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] For each of your Digimon with the [Deva]/[Four Sovereigns] trait, suspend 1 of your opponent's Digimon. Then, during your opponent's next unsuspend phase, all of their Digimon can't unsuspend .
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX5-041 Suspend Digimons and opponent's Digimon can't unsuspend")
        effect2.set_effect_description("[On Play] For each of your Digimon with the [Deva]/[Four Sovereigns] trait, suspend 1 of your opponent's Digimon. Then, during your opponent's next unsuspend phase, all of their Digimon can't unsuspend .")
        effect2.is_on_play = True
        effect2._is_cannot_unsuspend_player = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend Player, Grant Cannot Unsuspend, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Deva' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Four Sovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('FourSovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend_player')
            # Prevent target from unsuspending
            if not (player and game):
                return
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            def on_freeze(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_UNSUSPEND, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_freeze, filter_fn=lambda p: p.is_suspended, is_optional=False)
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] For each of your Digimon with the [Deva]/[Four Sovereigns] trait, suspend 1 of your opponent's Digimon. Then, during your opponent's next unsuspend phase, all of their Digimon can't unsuspend .
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX5-041 Suspend Digimons and opponent's Digimon can't unsuspend")
        effect3.set_effect_description("[When Digivolving] For each of your Digimon with the [Deva]/[Four Sovereigns] trait, suspend 1 of your opponent's Digimon. Then, during your opponent's next unsuspend phase, all of their Digimon can't unsuspend .")
        effect3.is_when_digivolving = True
        effect3._is_cannot_unsuspend_player = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend Player, Grant Cannot Unsuspend, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Deva' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Four Sovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('FourSovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend_player')
            # Prevent target from unsuspending
            if not (player and game):
                return
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            def on_freeze(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_UNSUSPEND, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_freeze, filter_fn=lambda p: p.is_suspended, is_optional=False)
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Delete 1 of your opponent's suspended Digimon.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("EX5-041 Delete 1 suspended Digimon")
        effect4.set_effect_description("[On Deletion] Delete 1 of your opponent's suspended Digimon.")
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
