from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_036(CardScript):
    """BT14-036 Centarumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-036 DP -3000")
        effect0.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect0.is_when_digivolving = True
        effect0.dp_modifier = -3000

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP -3000"""
            player = ctx.get('player')
            enemy = player.enemy if player else None
            if not enemy or not enemy.battle_area:
                return

            dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
            if not dp_targets:
                return

            chosen = ctx.get('target_permanent') or ctx.get('target')
            if chosen not in dp_targets:
                # Deterministic fallback when no explicit target is provided by engine/UI.
                chosen = min(dp_targets, key=lambda p: p.dp)
            chosen.change_dp(-3000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-036 DP -2000")
        effect1.set_effect_description("[When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-2000_BT14_036")
        effect1.is_on_attack = True
        effect1.dp_modifier = -2000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -2000"""
            player = ctx.get('player')
            enemy = player.enemy if player else None
            if not enemy or not enemy.battle_area:
                return

            dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
            if not dp_targets:
                return

            chosen = ctx.get('target_permanent') or ctx.get('target')
            if chosen not in dp_targets:
                # Deterministic fallback when no explicit target is provided by engine/UI.
                chosen = min(dp_targets, key=lambda p: p.dp)
            chosen.change_dp(-2000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
