from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_013(CardScript):
    """EX5-013 Zhuqiaomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-013 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-013 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] By deleting 1 Digimon with the [Deva] trait or 6000 DP or less, this Digimon gains <Security Attack +1> (This Digimon checks 1 additional security card) for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-013 Delete 1 Digimon to gain Security Attack +1")
        effect2.set_effect_description("[When Digivolving] [Once Per Turn] By deleting 1 Digimon with the [Deva] trait or 6000 DP or less, this Digimon gains <Security Attack +1> (This Digimon checks 1 additional security card) for the turn.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("DeleteAndGainSAttak_EX5_013")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] By deleting 1 Digimon with the [Deva] trait or 6000 DP or less, this Digimon gains <Security Attack +1> (This Digimon checks 1 additional security card) for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-013 Delete 1 Digimon to gain Security Attack +1")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] By deleting 1 Digimon with the [Deva] trait or 6000 DP or less, this Digimon gains <Security Attack +1> (This Digimon checks 1 additional security card) for the turn.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("DeleteAndGainSAttak_EX5_013")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Delete 1 of your opponent's Digimon with the highest DP.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX5-013 Delete opponent's 1 Digimon with the highest DP")
        effect4.set_effect_description("[On Deletion] Delete 1 of your opponent's Digimon with the highest DP.")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
