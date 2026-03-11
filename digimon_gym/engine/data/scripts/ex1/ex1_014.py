from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_014(CardScript):
    """EX1-014 ExVeemon | Lv.4 Digimon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Jamming> (unconditional keyword on this card)
        effect0 = ICardEffect()
        effect0.set_effect_name("EX1-014 Jamming")
        effect0.set_effect_description("<Jamming>")
        effect0._is_jamming = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Inherited] [Your Turn] While this Digimon has [Imperialdramon] in its name
        # or the [Free] trait, it gains <Jamming>.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX1-014 Jamming (inherited)")
        effect1.set_effect_description(
            "[Your Turn] While this Digimon has [Imperialdramon] in its name or the "
            "[Free] trait, it gains <Jamming>."
        )
        effect1._is_jamming = True
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            top = perm.top_card
            if top is None:
                return False
            if perm.contains_card_name('Imperialdramon'):
                return True
            if any('Free' in tr for tr in getattr(top, 'card_traits', [])):
                return True
            return False

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
