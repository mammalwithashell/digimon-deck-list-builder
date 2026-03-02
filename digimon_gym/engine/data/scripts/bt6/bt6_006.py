from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_006(CardScript):
    """BT6-006 Tsunomon | Lv.2 Purple DigiEgg

    Inherited Effect [Your Turn] [Once Per Turn] When you trash a card in
    your hand using one of your effects, Draw 1.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [Your Turn] [Once Per Turn] Draw on hand trash ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDiscardHand)
        effect0.set_effect_name("BT6-006 Inherited: draw on hand trash")
        effect0.set_effect_description(
            "[Your Turn] [Once Per Turn] When you trash a card in your hand "
            "using one of your effects, Draw 1."
        )
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("YourTurn_BT6-006_DrawOnTrash")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Draw 1 card when a hand card is trashed by effect."""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
