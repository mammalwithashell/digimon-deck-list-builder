from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_086(CardScript):
    """BT20-086 Altea"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # Set memory to 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-086 Set memory to 3")
        effect0.set_effect_description("Set memory to 3")
        # [Start of Your Turn] Set memory to 3 if <= 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By placing 1 play cost 4 or lower black Digimon card with the [Cyborg] or [Machine] trait from your hand or trash as any of your [Cyborg] or [Machine] trait Digimon's bottom digivolution card, flip your opponent's top face-down security card face up.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-086 By placing a card as bottom digivolution source, flip opponents top security face up")
        effect1.set_effect_description("[Start of Your Main Phase] By placing 1 play cost 4 or lower black Digimon card with the [Cyborg] or [Machine] trait from your hand or trash as any of your [Cyborg] or [Machine] trait Digimon's bottom digivolution card, flip your opponent's top face-down security card face up.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-086 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
