from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_046(CardScript):
    """EX1-046 Kurisarimon | Lv.4 (White, Cost 4)

    Inherited Effect [Your Turn] [Once Per Turn] When one of your other
        Digimon with the same name as this Digimon is deleted, unsuspend
        this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited: [Your Turn] [OPT] When same-name Digimon deleted, unsuspend
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("EX1-046 Inherited: Unsuspend on same-name deletion")
        effect0.set_effect_description(
            "[Your Turn] [Once Per Turn] When one of your other Digimon with "
            "the same name as this Digimon is deleted, unsuspend this Digimon."
        )
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("EX1_046_Unsuspend")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            # Check if the deleted permanent was ours and had the same name
            own_perm = card.permanent_of_this_card()
            deleted_perm = context.get('deleted_permanent') or context.get('event_permanent')
            if not deleted_perm:
                return False
            if deleted_perm is own_perm:
                return False  # must be ANOTHER Digimon
            # Check same name
            if own_perm and own_perm.top_card and deleted_perm.top_card:
                own_name = own_perm.top_card.card_names[0] if own_perm.top_card.card_names else None
                del_name = deleted_perm.top_card.card_names[0] if deleted_perm.top_card.card_names else None
                if own_name and del_name and own_name == del_name:
                    return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            perm = card.permanent_of_this_card() if card else None
            if perm and perm.is_suspended:
                perm.unsuspend()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
