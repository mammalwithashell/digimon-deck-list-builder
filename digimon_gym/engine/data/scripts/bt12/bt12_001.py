from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT12_001(CardScript):
    """BT12-001 Gigimon | Lv.2 Digi-Egg (Red)

    Inherited Effect [Your Turn] Add 1000 to the maximum DP you can
    choose with DP-based deletion effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Inherited: [Your Turn] +1000 to max DP of DP-based deletion effects ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT12-001 Inherited: +1000 DP deletion threshold")
        effect0.set_effect_description(
            "Inherited Effect [Your Turn] Add 1000 to the maximum DP you "
            "can choose with DP-based deletion effects."
        )
        effect0.is_inherited_effect = True
        effect0.is_declarative = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # [Your Turn] only
            if not owner.is_my_turn:
                return False
            # Must be on owner's battle area as a Digimon
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            if perm not in owner.battle_area:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        # The engine checks dp_delete_max_bonus on inherited effects
        # to raise the DP threshold for DP-based deletion.
        # +1000 to the owner's DP-based deletion max when sourced by owner.
        effect0._dp_delete_max_bonus = 1000

        effects.append(effect0)

        return effects
