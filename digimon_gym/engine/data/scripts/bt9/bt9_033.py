from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_033(CardScript):
    """BT9-033 Pillomon | Lv.3 Yellow Digimon

    [All Turns] Players can't play Digimon by effects.

    NOTE: This is a restrictive continuous effect. The engine does not
    currently have a built-in mechanism to prevent effect-based plays.
    Implemented as a descriptive-tagged effect that marks the condition.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] Players can't play Digimon by effects ---
        # This is a continuous lock effect. Since the engine doesn't fully
        # support blocking effect-based plays, we tag it descriptively.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT9-033 Players can't play Digimon by effects")
        effect0.set_effect_description(
            "[All Turns] Players can't play Digimon by effects."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            # Continuous effect — active while Pillomon is on the field
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        # descriptive-tagged: engine lacks play-lock mechanism
        effects.append(effect0)

        return effects
