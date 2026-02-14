from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_014(CardScript):
    """BT24-014 Aegiochusmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-014 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Aegiomon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Aegiomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Aegiomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-014 Security Attack +1")
        effect1.set_effect_description("Security Attack +1")
        effect1._security_attack_modifier = 1

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -5000 DP for the turn. Then, if you have 3 or fewer security cards, delete 1 of your opponent's Digimon with 7000 DP or less.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-014 Give -5000 DP, then delete 1 Digimon with 7000 DP or less")
        effect2.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -5000 DP for the turn. Then, if you have 3 or fewer security cards, delete 1 of your opponent's Digimon with 7000 DP or less.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -5000, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-5000)
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 7000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
