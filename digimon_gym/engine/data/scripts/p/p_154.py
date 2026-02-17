from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_154(CardScript):
    """P-154 Maildramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When another of your Digimon with [Knightmon] in its text would leave the battle area by an opponent's effect, by placing this Digimon as its bottom digivolution card, it doesn't leave.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-154 Add as bottom source to prevent deletion")
        effect0.set_effect_description("[All Turns] When another of your Digimon with [Knightmon] in its text would leave the battle area by an opponent's effect, by placing this Digimon as its bottom digivolution card, it doesn't leave.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Knightmon' in text):
                    return False
            else:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("P-154 Blocker")
        effect1.set_effect_description("Blocker")
        effect1.is_inherited_effect = True
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
