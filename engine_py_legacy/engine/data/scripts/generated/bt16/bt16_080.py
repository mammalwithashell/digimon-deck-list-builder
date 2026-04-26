from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_080(CardScript):
    """BT16-080 Shroudmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-080 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 5
        effect0._alt_digi_cost = 5

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Pulsemon' in text):
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area by your opponent's effects, if you have 3 or more security cards, by trashing the top card of your security stack, prevent it from leaving.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-080 Trash a security to prevent this Digimon from leaving Battle Area")
        effect1.set_effect_description("[All Turns] When this Digimon would leave the battle area by your opponent's effects, if you have 3 or more security cards, by trashing the top card of your security stack, prevent it from leaving.")
        effect1.is_optional = True
        effect1.set_hash_string("TrashSecurityToStay_BT16_080")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] <Recovery +1> until you have 3 security cards.)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-080 Recovery +1 until you have 3 security cards.")
        effect2.set_effect_description("[On Deletion] <Recovery +1> until you have 3 security cards.)")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
