from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_036(CardScript):
    """EX11-036 Dalphomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-036 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: vortex
        # Vortex
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-036 Vortex")
        effect1.set_effect_description("Vortex")
        effect1._is_vortex = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Suspend, Gain Keyword Cannot Unsuspend
        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-036 Suspend, Gain Keyword Cannot Unsuspend")
        effect2.set_effect_description("Suspend, Gain Keyword Cannot Unsuspend")
        effect2.is_on_play = True
        effect2._is_cannot_unsuspend = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend"""
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Suspend, Gain Keyword Cannot Unsuspend
        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-036 Suspend, Gain Keyword Cannot Unsuspend")
        effect3.set_effect_description("Suspend, Gain Keyword Cannot Unsuspend")
        effect3.is_when_digivolving = True
        effect3._is_cannot_unsuspend = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend"""
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

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAllyAttack
        # Suspend, Gain Keyword Cannot Unsuspend
        effect4 = ICardEffect()
        effect4.set_effect_name("EX11-036 Suspend, Gain Keyword Cannot Unsuspend")
        effect4.set_effect_description("Suspend, Gain Keyword Cannot Unsuspend")
        effect4.is_on_attack = True
        effect4._is_cannot_unsuspend = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend"""
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

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # Digivolve
        effect5 = ICardEffect()
        effect5.set_effect_name("EX11-036 1 other digimon may digivolve into a black card w/[Maquinamon] in text for free")
        effect5.set_effect_description("Digivolve")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX11_036_EOYT")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not ('Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.None
        # Effect
        effect6 = ICardEffect()
        effect6.set_effect_name("EX11-036 Effect")
        effect6.set_effect_description("Effect")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        # Timing: EffectTiming.WhenLinked
        # Suspend, Force Attack
        effect7 = ICardEffect()
        effect7.set_effect_name("EX11-036 Suspend 1 Digimon, then this may attack")
        effect7.set_effect_description("Suspend, Force Attack")
        effect7.is_inherited_effect = True
        effect7.is_optional = True
        effect7.set_max_count_per_turn(1)
        effect7.set_hash_string("EX11_036_YT_ESS")

        effect = effect7  # alias for condition closure
        def condition7(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect7.set_can_use_condition(condition7)

        def process7(ctx: Dict[str, Any]):
            """Action: Suspend, Force Attack"""
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
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect7.set_on_process_callback(process7)
        effects.append(effect7)

        return effects
