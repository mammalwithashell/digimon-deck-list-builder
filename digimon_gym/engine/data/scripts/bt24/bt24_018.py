from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_018(CardScript):
    """Auto-transpiled from DCGO BT24_018.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-018 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: armor_purge
        # Armor Purge
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-018 Armor Purge")
        effect1.set_effect_description("Armor Purge")
        effect1._is_armor_purge = True
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-018 May trash 1 opponent's security. Then, this may unsuspend.")
        effect2.set_effect_description("Effect")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # Delete
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-018 Delete 1 of your opponent's Digimon?")
        effect3.set_effect_description("Delete")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_18_AT_Sec_Removed")

        def condition3(context: Dict[str, Any]) -> bool:
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

        # Timing: EffectTiming.WhenRemoveField
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-018 Delte an opponent's Digimon, to prevent [Reptile] or [Dragonkin] trait digimon from leaving the battle area")
        effect4.set_effect_description("Effect")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_018_AT_Prevent_Deletion")

        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
