from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_070(CardScript):
    """BT19-070 Kimeramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-070 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-070 Effect")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By deleting 1 of your Digimon, delete 1 of your opponent's level 3 Digimon, 1 of their level 4 Digimon, and 1 of their level 5 Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-070 Delete 1 of your Digimon to delete your opponent's Digimon")
        effect2.set_effect_description("[On Play] By deleting 1 of your Digimon, delete 1 of your opponent's level 3 Digimon, 1 of their level 4 Digimon, and 1 of their level 5 Digimon.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
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
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By deleting 1 of your Digimon, delete 1 of your opponent's level 3 Digimon, 1 of their level 4 Digimon, and 1 of their level 5 Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-070 Delete 1 of your Digimon to delete your opponent's Digimon")
        effect3.set_effect_description("[When Digivolving] By deleting 1 of your Digimon, delete 1 of your opponent's level 3 Digimon, 1 of their level 4 Digimon, and 1 of their level 5 Digimon.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
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
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] By deleting 1 of your level 4 or lower purple or red Digimon, you may play 1 [Machinedramon] from your trash without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT19-070 Delete 1 of your Digimon to play [Machinedramon] from your trash")
        effect4.set_effect_description("[On Deletion] By deleting 1 of your level 4 or lower purple or red Digimon, you may play 1 [Machinedramon] from your trash without paying the cost.")
        effect4.is_optional = True
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Machinedramon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect5 = ICardEffect()
        effect5.set_effect_name("BT19-070 Security Attack +1")
        effect5.set_effect_description("Security Attack +1")
        effect5.is_inherited_effect = True
        effect5._security_attack_modifier = 1

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
