from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

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

        # Factory effect: decode
        # Decode
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-014 Decode")
        effect2.set_effect_description("Decode")
        effect2._is_decode = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -5000 DP for the turn. Then, if you have 3 or fewer security cards, delete 1 of your opponent's Digimon with 7000 DP or less.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-014 Give -5000 DP, then delete 1 Digimon with 7000 DP or less")
        effect3.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -5000 DP for the turn. Then, if you have 3 or fewer security cards, delete 1 of your opponent's Digimon with 7000 DP or less.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -5000 to 1 opponent Digimon, then delete 1 with 7000 DP or less"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: Select 1 opponent Digimon to give -5000 DP
            def dp_filter(p):
                return p.is_digimon and p.dp is not None

            def on_dp_target(target_perm):
                if target_perm:
                    target_perm.change_dp(-5000)

                # Step 2: Then, if 3 or fewer security cards, delete 1 opponent Digimon with 7000 DP or less
                if len(player.security_cards) > 3:
                    return
                def delete_filter(p):
                    if p.dp is None or p.dp > 7000:
                        return False
                    return p.is_digimon
                def on_delete(del_perm):
                    if del_perm and enemy:
                        enemy.delete_permanent(del_perm)
                if any(delete_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=delete_filter, is_optional=False,
                        prompt="Delete 1 of your opponent's Digimon with 7000 DP or less.")

            if any(dp_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_dp_target, filter_fn=dp_filter, is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to give -5000 DP.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: decode
        # Decode
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-014 Decode")
        effect4.set_effect_description("Decode")
        effect4.is_inherited_effect = True
        effect4._is_decode = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
