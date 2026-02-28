from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_008(CardScript):
    """BT13-008 Agumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-008 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Main][Once Per Turn] For the turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon and can't digivolve.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-008 Your 1 [Marcus Damon] becomes Digimon")
        effect1.set_effect_description("[Main][Once Per Turn] For the turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon and can't digivolve.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BecomeDigimon_BT13_008")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-008 Delete 1 Digimon with 3000 DP or less")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Delete_BT13_008")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 3000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
