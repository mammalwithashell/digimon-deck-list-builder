from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_109(CardScript):
    """BT11-109 Astral Snatcher"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may place up to 3 Digimon cards with [Bagra Army] in their traits from your trash under 1 of your Digimon as its bottom digivolution cards or under 1 of your Tamers. Then, if you have a Digimon or Tamer with [Bagra Army] in its traits in play, place 1 of your opponent's Digimon under 1 of your opponent's other Digimon as its bottom digivolution card.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-109 Effect")
        effect0.set_effect_description("[Main] You may place up to 3 Digimon cards with [Bagra Army] in their traits from your trash under 1 of your Digimon as its bottom digivolution cards or under 1 of your Tamers. Then, if you have a Digimon or Tamer with [Bagra Army] in its traits in play, place 1 of your opponent's Digimon under 1 of your opponent's other Digimon as its bottom digivolution card.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-109 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
