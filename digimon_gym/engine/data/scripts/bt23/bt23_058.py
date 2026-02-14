from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_058(CardScript):
    """BT23-058"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-058 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: reboot
        # Reboot
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-058 Reboot")
        effect1.set_effect_description("Reboot")
        effect1._is_reboot = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-058 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon or Tamers would leave the battle area by your opponent's effects, by suspending this Digimon, 1 of those Digimon or Tamers doesn't leave.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-058 Prevent Digimon from leaving play")
        effect3.set_effect_description("[All Turns] When any of your Digimon or Tamers would leave the battle area by your opponent's effects, by suspending this Digimon, 1 of those Digimon or Tamers doesn't leave.")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
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
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon suspends, delete all of your opponent's Digimon with the lowest play cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-058 Delete opponent's Digimon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspends, delete all of your opponent's Digimon with the lowest play cost.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("AT_BT23-058")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
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
