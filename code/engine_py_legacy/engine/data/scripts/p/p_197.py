from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_197(CardScript):
    """P-197 Patamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-197 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [TS] trait for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_trait = "TS"
        effect0._alt_digi_level = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If you have 4 or less memory, this Digimon may digivolve into a Digimon card with the [Angel] or [TS] trait in the hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("P-197 Digivolve into a [Angel]/[TS] digimon in hand")
        effect1.set_effect_description("[Start of Your Main Phase] If you have 4 or less memory, this Digimon may digivolve into a Digimon card with the [Angel] or [TS] trait in the hand without paying the cost.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Only activates if player has 4 or less memory
            player = card.owner if card else None
            if player is None:
                return False
            game = getattr(player, 'game', None)
            if game is not None and game.memory > 4:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('Angel' in _t or 'TS' in _t for _t in traits):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, cost_override=0, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("P-197 -2k DP")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("P_196_DPMinus")
        effect2.is_on_attack = True
        effect2.dp_modifier = -2000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -2000 to 1 opponent Digimon (player selects)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def dp_filter(p):
                return p.is_digimon and p.dp is not None

            def on_dp_target(target_perm):
                if target_perm:
                    target_perm.change_dp(-2000)

            if any(dp_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_dp_target, filter_fn=dp_filter, is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to give -2000 DP.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
