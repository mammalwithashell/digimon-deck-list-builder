from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_042(CardScript):
    """BT24-042 Goblimon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-042 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Tsunomon] for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Tsunomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Tsunomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: change_digi_cost
        # Change digivolution cost
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-042 Change digivolution cost")
        effect1.set_effect_description("Change digivolution cost")
        # Reduce digivolution cost by 1 for matching
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn] [Once Per Turn] When your hand is trashed from, this [Demon] or [Titan] trait Digimon may digivolve into [Titamon] or a [Titan] trait Digimon card in the trash with the digivolution cost reduced by 1.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-042 When your hand is trashed from, digivolve")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When your hand is trashed from, this [Demon] or [Titan] trait Digimon may digivolve into [Titamon] or a [Titan] trait Digimon card in the trash with the digivolution cost reduced by 1.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_042_YT_ESS")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
