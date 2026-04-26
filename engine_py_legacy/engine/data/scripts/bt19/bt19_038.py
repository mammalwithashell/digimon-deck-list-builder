from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_038(CardScript):
    """BT19-038 JaegerDorulumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-038 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Xros Heart] trait for cost 3
        effect0._alt_digi_cost = 3
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
        effect1.set_effect_name("BT19-038 Also treated as [Dorulumon] for a DigiXros")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 1 of your opponent's Digimon. Then, 1 of their Digimon can't activate [When Digivolving] effects or unsuspend until the end of their turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT19-038 Suspend 1 Digimon and 1 Digimon cannot activate [When Digivolving] effects or unsuspend")
        effect2.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon. Then, 1 of their Digimon can't activate [When Digivolving] effects or unsuspend until the end of their turn.")
        effect2.set_max_count_per_turn(1)
        effect2.is_on_play = True
        effect2._is_cannot_unsuspend = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend, Disable Effect, Grant Cannot Unsuspend, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend')
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Prevent target from unsuspending
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_freeze(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_UNSUSPEND, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_freeze, filter_fn=lambda p: p.is_suspended, is_optional=False)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon. Then, 1 of their Digimon can't activate [When Digivolving] effects or unsuspend until the end of their turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT19-038 Suspend 1 Digimon and 1 Digimon cannot activate [When Digivolving] effects or unsuspend")
        effect3.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon. Then, 1 of their Digimon can't activate [When Digivolving] effects or unsuspend until the end of their turn.")
        effect3.set_max_count_per_turn(1)
        effect3.is_when_digivolving = True
        effect3._is_cannot_unsuspend = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend, Disable Effect, Grant Cannot Unsuspend, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend')
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Prevent target from unsuspending
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_freeze(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_UNSUSPEND, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_freeze, filter_fn=lambda p: p.is_suspended, is_optional=False)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 Digimon card with the [Xros Heart]/[Blue Flare] trait from your hand or trash under your Tamers.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT19-038 Place 1 [Xros Heart]/[Blue Flare] card from trash under 1 of your Tamers, then <Save>")
        effect4.set_effect_description("[On Deletion] Place 1 Digimon card with the [Xros Heart]/[Blue Flare] trait from your hand or trash under your Tamers.")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
