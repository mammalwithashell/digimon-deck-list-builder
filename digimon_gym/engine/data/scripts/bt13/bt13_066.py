from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_066(CardScript):
    """BT13-066 Dorugamon | Lv.4"""

    def get_card_effects(self, card: "CardSource") -> List["ICardEffect"]:
        effects = []

        # Factory effect: inherited dp modifier
        # [All Turns] While this Digimon has the [X Antibody] trait, it gets +1000 DP.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-066 Inherited +1000 DP with X Antibody")
        effect0.set_effect_description(
            "[All Turns] While this Digimon has the [X Antibody] trait, it gets +1000 DP."
        )
        effect0.is_inherited_effect = True
        effect0.dp_modifier = 1000

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (
                permanent
                and permanent.top_card
                and any(
                    "X Antibody" in trait
                    for trait in (getattr(permanent.top_card, "card_traits", []) or [])
                )
            ):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
