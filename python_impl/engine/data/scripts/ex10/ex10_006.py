from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_006(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # 2. OnStartMainPhase
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-006: Recycle Greymon")
        effect1.set_effect_description("[Start of Your Main Phase] You may return 1 [Virus] trait Digimon card with [Greymon] in its name from your trash to the hand.")
        effect1.timing = EffectTiming.OnStartMainPhase

        def condition1(context: Dict[str, Any]) -> bool:
            return card.owner and card.owner.is_my_turn

        effect1.set_can_use_condition(condition1)

        def activate1():
            # Logic to select and return.
            # Simplified: just return first valid one.
            if not card.owner:
                return

            trash = card.owner.trash_cards
            target = None
            for c in trash:
                # Basic check
                name_match = any("Greymon" in name for name in c.card_names)
                trait_match = "Virus" in c.card_traits
                if name_match and trait_match:
                    target = c
                    break
            if target:
                card.owner.trash_cards.remove(target)
                card.owner.hand_cards.append(target)
                print(f"Returned {target.card_names[0]} to hand.")

        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # 3. ESS: +1000 DP
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-006 ESS: +1000 DP")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 1000

        def condition2(context: Dict[str, Any]) -> bool:
            permanent = context.get("permanent")
            if permanent and permanent.top_card.owner.is_my_turn:
                return True
            return False

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
