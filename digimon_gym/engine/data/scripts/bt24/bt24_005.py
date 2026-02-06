from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_005(CardScript):
    """Auto-transpiled from DCGO BT24_005.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn] [Once Per Turn] When Tamer cards are placed in this Digimon's digivolution cards, reveal the top 3 cards of your deck. Return the revealed cards to the top or bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-005 Reveal 3 then return to top or bot.")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When Tamer cards are placed in this Digimon's digivolution cards, reveal the top 3 cards of your deck. Return the revealed cards to the top or bottom of the deck.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT24_005_Reveal")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
