from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_002(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("BT20-002 Inherited Effect")
        effect.set_effect_description("[When Attacking] [Once Per Turn] If this Digimon has [Dracomon] or [Examon] in its text, <Draw 1>.")
        effect.is_inherited_effect = True
        effect.is_on_attack = True
        effect.set_max_count_per_turn(1)

        def condition(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent
            if not permanent or not permanent.top_card:
                return False

            top = permanent.top_card
            keywords = ["Dracomon", "Examon"]

            # Check names
            for name in top.card_names:
                for kw in keywords:
                    if kw in name:
                        return True

            # Check traits
            for trait in top.card_traits:
                for kw in keywords:
                    if kw in trait:
                        return True

            # Check effect text if available
            if top.c_entity_base:
                text = top.c_entity_base.effect_description_eng
                for kw in keywords:
                    if kw in text:
                        return True

            return False

        def on_process():
            if card.owner:
                card.owner.draw()

        effect.set_can_use_condition(condition)
        effect.set_on_process_callback(on_process)

        return [effect]
