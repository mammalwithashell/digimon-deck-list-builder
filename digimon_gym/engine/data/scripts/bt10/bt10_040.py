from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_040(CardScript):
    """BT10-040 Achillesmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 2 or fewer security cards, <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT10-040 Recovery +1 (Deck)")
        effect0.set_effect_description("[When Digivolving] If you have 2 or fewer security cards, <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking][Once Per Turn] If you have 3 or more security cards, 1 of your opponent's Digimon gets -5000 DP for the turn. If you have 3 or fewer security cards, gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT10-040 Opponent's 1 Digimon gets DP -5000 or get Memory +2")
        effect1.set_effect_description("[When Attacking][Once Per Turn] If you have 3 or more security cards, 1 of your opponent's Digimon gets -5000 DP for the turn. If you have 3 or fewer security cards, gain 2 memory.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-5000Memory+1_BT10_040")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 2 memory, DP -5000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-5000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
