from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_002(CardScript):
    """EX4-002 Kokomon | Digi-Egg Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When an effect suspends one of your Digimon, <Draw 1>
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnTappedAnyone)
        effect0.set_effect_name("EX4-002 Draw 1 when your Digimon is suspended")
        effect0.set_effect_description(
            "[Your Turn][Once Per Turn] When an effect suspends one of your Digimon, "
            "<Draw 1>"
        )
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw1_EX4_002")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The suspended permanent must be one of the owner's Digimon
            suspended_perm = context.get('permanent')
            if suspended_perm is None:
                return False
            owner = card.owner if card else None
            if owner is None:
                return False
            if suspended_perm not in owner.battle_area:
                return False
            if not suspended_perm.is_digimon:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
