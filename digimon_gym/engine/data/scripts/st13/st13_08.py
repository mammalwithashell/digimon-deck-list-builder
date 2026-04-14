from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST13_08(CardScript):
    """ST13-08 Chikurimon | Lv.3 Black Rookie Digimon (3000 DP, Cost 3)

    [All Turns] Players can't reduce play costs.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] Players can't reduce play costs ---
        # Declarative passive: engine queries _blocks_cost_reduction attribute
        # on permanent effects during calculate_play_cost. This works whether
        # the Chikurimon was played normally or injected via place_on_field.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.NoTiming)
        effect0.set_effect_name("ST13-08 Players can't reduce play costs")
        effect0.set_effect_description("[All Turns] Players can't reduce play costs.")
        effect0.is_declarative = True
        effect0._blocks_cost_reduction = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
