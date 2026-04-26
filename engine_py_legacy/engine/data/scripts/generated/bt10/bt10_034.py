from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_034(CardScript):
    """BT10-034 Dorulumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-034 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have another Digimon or Tamer with [Xros Heart] in its traits in play, 1 of your opponent's Digimon gets -3000 DP until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-034 DP -3000")
        effect1.set_effect_description("[On Play] If you have another Digimon or Tamer with [Xros Heart] in its traits in play, 1 of your opponent's Digimon gets -3000 DP until the end of your opponent's turn.")
        effect1.is_on_play = True
        effect1.dp_modifier = -3000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -3000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: save
        # Save
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-034 Save")
        effect2.set_effect_description("Save")
        effect2.is_on_deletion = True
        effect2._is_save = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
