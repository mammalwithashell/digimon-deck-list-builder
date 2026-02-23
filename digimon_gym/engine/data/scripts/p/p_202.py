from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_202(CardScript):
    """P-202 Tyrannomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-202 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: training
        # Training
        effect1 = ICardEffect()
        effect1.set_effect_name("P-202 Training")
        effect1.set_effect_description("Training")
        effect1._is_training = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.BeforePayCost
        # [Your Turn] [Once Per Turn] When any of your suspended Digimon would digivolve into a Digimon card with [Tyrannomon] in its name or the [Dinosaur] or [Ver.1] trait, reduce the digivolution cost by 1.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-202 Digivolution Cost -1")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When any of your suspended Digimon would digivolve into a Digimon card with [Tyrannomon] in its name or the [Dinosaur] or [Ver.1] trait, reduce the digivolution cost by 1.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("P_202_YT")
        effect2.cost_reduction = 1

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
