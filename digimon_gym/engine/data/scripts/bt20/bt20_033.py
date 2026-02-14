from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_033(CardScript):
    """BT20-033 LoaderLeomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-033 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [ACCEL] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "ACCEL"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('ACCEL' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Until the end of your opponent's turn, 1 of their Digimon can't activate [When Digivolving] effects and gets -3000 DP.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-033 1 Opponent's Digimon gets can't activate [When Digivolving] and -3000 DP")
        effect1.set_effect_description("[On Play] Until the end of your opponent's turn, 1 of their Digimon can't activate [When Digivolving] effects and gets -3000 DP.")
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until the end of your opponent's turn, 1 of their Digimon can't activate [When Digivolving] effects and gets -3000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-033 1 Opponent's Digimon gets can't activate [When Digivolving] and -3000 DP")
        effect2.set_effect_description("[When Digivolving] Until the end of your opponent's turn, 1 of their Digimon can't activate [When Digivolving] effects and gets -3000 DP.")
        effect2.is_when_digivolving = True
        effect2.dp_modifier = -3000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn] [Once Per Turn] When any of your opponent's Digimon attack, you may change the attack target to this Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-033 You may change the attack target to this Digimon.")
        effect3.set_effect_description("[Opponent's Turn] [Once Per Turn] When any of your opponent's Digimon attack, you may change the attack target to this Digimon.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Redirect_BT20-033")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
