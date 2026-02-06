from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_015(CardScript):
    """Auto-transpiled from DCGO BT24_015.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-015 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] If your opponent has a level 6 or higher Digimon, play this card without battling and without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-015 Play this card without battling")
        effect1.set_effect_description("[Security] If your opponent has a level 6 or higher Digimon, play this card without battling and without paying the cost.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [All Turns] [Once Per Turn] When attack targets change, delete 1 of your opponent's Digimon with the lowest DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-015 Delete lowest DP")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When attack targets change, delete 1 of your opponent's Digimon with the lowest DP.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("AT_BT24-015")

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Delete 1 of your opponent's Digimon with ＜Blocker＞
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-015 Delete 1 <Blocker>")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] Delete 1 of your opponent's Digimon with ＜Blocker＞")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WAESS_BT24-015")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
