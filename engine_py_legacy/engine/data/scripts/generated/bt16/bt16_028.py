from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_028(CardScript):
    """BT16-028 Imperialdramon: Dragon Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-028 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon or Tamer can't unsuspend until the end of their turn. Then, by suspending 1 of their Digimon or Tamers, unsuspend 1 of your Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-028 1 of your opponent's Digimon/Tamer can't unsuspend.")
        effect1.set_effect_description("[When Digivolving] 1 of your opponent's Digimon or Tamer can't unsuspend until the end of their turn. Then, by suspending 1 of their Digimon or Tamers, unsuspend 1 of your Digimon.")
        effect1.is_when_digivolving = True
        effect1._is_cannot_unsuspend = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Unsuspend, Gain Keyword Cannot Unsuspend, Grant Cannot Unsuspend"""
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
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend')
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When an effect plays or digivolves an opponent's Digimon, if you have a Tamer, this Digimon may digivolve into [Imperialdramon: Fighter Mode] in your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-028 Digivolve into [Imperialdramon: Fighter Mode]")
        effect2.set_effect_description("[All Turns] When an effect plays or digivolves an opponent's Digimon, if you have a Tamer, this Digimon may digivolve into [Imperialdramon: Fighter Mode] in your hand without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
