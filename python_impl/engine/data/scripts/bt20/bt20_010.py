from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_010(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Main Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-010 Main Effect")
        effect1.set_effect_description("[Your Turn] When this Digimon would digivolve into [Ginryumon] or a Digimon card with the [Chronicle] trait, reduce the digivolution cost by 1.")
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            player = context.get("player")
            card_source = context.get("card_source")

            if not player or not player.is_my_turn:
                return False
            if not card_source:
                return False

            # Check target: Ginryumon or Chronicle
            names = [n.lower() for n in card_source.card_names]
            if any("ginryumon" in n for n in names):
                return True

            if "Chronicle" in card_source.card_traits:
                return True

            return False

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Inherited Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-010 Inherited")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            perm = context.get("permanent")
            if perm and perm.top_card and perm.top_card.owner:
                return perm.top_card.owner.is_my_turn
            return False

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
