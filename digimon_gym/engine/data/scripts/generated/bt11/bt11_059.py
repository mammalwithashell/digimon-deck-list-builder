from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_059(CardScript):
    """BT11-059 RustTyrannomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: change_digi_cost
        # Change digivolution cost
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-059 Change digivolution cost")
        effect0.set_effect_description("Change digivolution cost")
        # Reduce digivolution cost by 1 for matching
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns][Once Per Turn] When this Digimon deletes an opponent's Digimon in battle, unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-059 Unsuspend this Digimon")
        effect1.set_effect_description("[All Turns][Once Per Turn] When this Digimon deletes an opponent's Digimon in battle, unsuspend this Digimon.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT11_059")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
