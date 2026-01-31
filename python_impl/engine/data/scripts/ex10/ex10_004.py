from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_004(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("EX10-004 ESS: On Move")
        effect.set_effect_description("[Your Turn] [Once Per Turn] When any of your Digimon with [Lucemon] in their names move from the breeding area to the battle area, by trashing 1 card in your hand, <Draw 1> and gain 1 memory.")
        effect.is_inherited_effect = True
        effect.timing = EffectTiming.OnMove
        effect.max_count_per_turn = 1

        def condition(context: Dict[str, Any]) -> bool:
            permanent = context.get("permanent")
            if permanent and "Lucemon" in permanent.top_card.card_names[0] and permanent.top_card.owner == card.owner:
                return True
            return False

        effect.set_can_use_condition(condition)

        def activate():
            owner = card.owner
            if owner and owner.hand_cards:
                # Trashing logic simplified: Trash last card
                trashed = owner.hand_cards.pop()
                owner.trash_cards.append(trashed)
                print(f"Trashed {trashed.card_names[0]}")

                owner.draw()
                owner.memory += 1
                print("Draw 1, Gain 1 Memory")

        effect.set_on_process_callback(activate)

        return [effect]
