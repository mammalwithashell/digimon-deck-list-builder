from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_053(CardScript):
    """BT14-053 Rosemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-053 Suspend 1 Digimon or Tamer")
        effect0.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend"""
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

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Suspend 1 of your opponent's Digimon or Tamers.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-053 Suspend 1 Digimon or Tamer")
        effect1.set_effect_description("[When Attacking] Suspend 1 of your opponent's Digimon or Tamers.")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend"""
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When an effect suspends a Digimon or Tamer, you may unsuspend this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-053 Unsuspend this Digimon")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When an effect suspends a Digimon or Tamer, you may unsuspend this Digimon.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Unsuspend_BT14_053")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
