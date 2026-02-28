from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_042(CardScript):
    """BT15-042 Magnadramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns][Once per turn] When a card is removed from your security stack, if you have 3 or less security cards, you may place 1 yellow card from your hand at the top or bottom of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-042 Place 1 yellow card from your hand to the top or bottom of your security stack.")
        effect0.set_effect_description("[All Turns][Once per turn] When a card is removed from your security stack, if you have 3 or less security cards, you may place 1 yellow card from your hand at the top or bottom of your security stack.")
        effect0.set_max_count_per_turn(1)

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -9000 DP until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-042 Trash the top or bottom of your security to reduce opponent's Digimon DP.")
        effect1.set_effect_description("[On Play] By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -9000 DP until the end of your opponent's turn.")
        effect1.is_optional = True
        effect1.set_hash_string("TrashSecuirtyToReduceDP_BT15_042")
        effect1.is_on_play = True
        effect1.dp_modifier = -9000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -9000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-9000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -9000 DP until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-042 Trash the top or bottom of your security to reduce opponent's Digimon DP.")
        effect2.set_effect_description("[When Digivolving] By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -9000 DP until the end of your opponent's turn.")
        effect2.is_optional = True
        effect2.set_hash_string("TrashSecuirtyToReduceDP_BT15_042")
        effect2.is_when_digivolving = True
        effect2.dp_modifier = -9000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -9000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-9000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
