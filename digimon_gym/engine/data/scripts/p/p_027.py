from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_027(CardScript):
    """P-027 MetalGarurumon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Digi-Burst 2> (Trash 2 of this Digimon's digivolution cards to activate the effect below.) - Use a purple Option card with a memory cost of 7 or less in your hand without paying its memory cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name("P-027 Use 1 Option from hand")
        effect0.set_effect_description("[Main] <Digi-Burst 2> (Trash 2 of this Digimon's digivolution cards to activate the effect below.) - Use a purple Option card with a memory cost of 7 or less in your hand without paying its memory cost.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
