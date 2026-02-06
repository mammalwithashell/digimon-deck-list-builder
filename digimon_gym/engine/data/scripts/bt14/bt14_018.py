from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_018(CardScript):
    """Auto-transpiled from DCGO BT14_018.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Play 1 [Amon of Crimson Flame] (Digimon/Red/6000 DP/<Rush>) Token and 1 [Umon of Blue Thunder] (Digimon/Yellow/6000 DP/<Blocker>) Token.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-018 Play tokens")
        effect0.set_effect_description("[On Play] Play 1 [Amon of Crimson Flame] (Digimon/Red/6000 DP/<Rush>) Token and 1 [Umon of Blue Thunder] (Digimon/Yellow/6000 DP/<Blocker>) Token.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Play 1 [Amon of Crimson Flame] (Digimon/Red/6000 DP/<Rush>) Token and 1 [Umon of Blue Thunder] (Digimon/Yellow/6000 DP/<Blocker>) Token.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-018 Play tokens")
        effect1.set_effect_description("[When Digivolving] Play 1 [Amon of Crimson Flame] (Digimon/Red/6000 DP/<Rush>) Token and 1 [Umon of Blue Thunder] (Digimon/Yellow/6000 DP/<Blocker>) Token.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.BeforePayCost
        # [All Turns] When this Digimon would digivolve or leave the battle area, delete all of your [Amon of Crimson Flame] and [Umon of Blue Thunder]. If this effect deletes, <Recovery +1 (Deck)>.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-018 Delete tokens and Recovery +1 (Deck)")
        effect2.set_effect_description("[All Turns] When this Digimon would digivolve or leave the battle area, delete all of your [Amon of Crimson Flame] and [Umon of Blue Thunder]. If this effect deletes, <Recovery +1 (Deck)>.")
        effect2.set_hash_string("Recovery1_BT14_018")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Recovery +1, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.recovery(1)
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would digivolve or leave the battle area, delete all of your [Amon of Crimson Flame] and [Umon of Blue Thunder]. If this effect deletes, <Recovery +1 (Deck)>.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-018 Delete tokens and Recovery +1 (Deck)")
        effect3.set_effect_description("[All Turns] When this Digimon would digivolve or leave the battle area, delete all of your [Amon of Crimson Flame] and [Umon of Blue Thunder]. If this effect deletes, <Recovery +1 (Deck)>.")
        effect3.set_hash_string("Recovery1_BT14_018")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Recovery +1, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.recovery(1)
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
