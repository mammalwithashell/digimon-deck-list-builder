from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_045(CardScript):
    """P-045 Kurisarimon | Lv.4 (White, Cost 4)

    Inherited Effect [All Turns] All of your other Digimon with the same name
        as this Digimon gain <Decoy (Black/White)>.

    NOTE: The engine's Decoy check uses has_keyword('_is_decoy') on a
    per-permanent basis and auto-activates. Since this is an inherited aura
    that conditionally grants Decoy to same-name Digimon, we mark this card
    itself with _is_decoy and describe the intended scope. The engine's
    existing decoy check already evaluates candidates in the owner's
    battle_area, so the simplest correct approach is to have this inherited
    effect declare _is_decoy on the host permanent — which means THIS
    Digimon acts as the Decoy sacrifice for other same-name Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited: This Digimon has _is_decoy (can sacrifice itself for
        # other Black/White Digimon with the same name).
        effect0 = ICardEffect()
        effect0.set_effect_name("P-045 Inherited: Decoy (Black/White)")
        effect0.set_effect_description(
            "[All Turns] All of your other Digimon with the same name as this "
            "Digimon gain <Decoy (Black/White)>."
        )
        effect0.is_inherited_effect = True
        effect0._is_decoy = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
