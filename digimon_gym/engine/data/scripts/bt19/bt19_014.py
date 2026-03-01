from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_014(CardScript):
    """BT19-014 Shoutmon EX6 | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alliance
        # Alliance
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-014 Alliance")
        effect0.set_effect_description("Alliance")
        effect0.is_on_attack = True
        effect0._is_alliance = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: reboot
        # Reboot
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-014 Reboot")
        effect1.set_effect_description("Reboot")
        effect1._is_reboot = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: save
        # Save
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-014 Save")
        effect2.set_effect_description("Save")
        effect2._is_save = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: material_save
        # Material Save
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-014 Material Save")
        effect3.set_effect_description("Material Save")
        effect3._is_material_save = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] For each color in this Digimon's digivolution cards, all of your opponent's Digimon get -1000 DP for the turn. Then, you may play 1 [ShootingStarmon] from under your Tamers without paying the cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT19-014 All of your opponent's Digimon get -DP for the turn then, play 1 [ShootingStarmon] from under your Tamers")
        effect4.set_effect_description("[On Play] For each color in this Digimon's digivolution cards, all of your opponent's Digimon get -1000 DP for the turn. Then, you may play 1 [ShootingStarmon] from under your Tamers without paying the cost.")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Delete 1 of your opponent's Digimon with as much or less DP than this Digimon.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnAllyAttack)
        effect5.set_effect_name("BT19-014 Delete 1 Digimon")
        effect5.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with as much or less DP than this Digimon.")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
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

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.None
        # Effect
        effect6 = ICardEffect()
        effect6.set_effect_name("BT19-014 Effect")
        effect6.set_effect_description("Effect")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
