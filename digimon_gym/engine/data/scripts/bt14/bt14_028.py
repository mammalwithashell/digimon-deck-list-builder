from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_028(CardScript):
    """BT14-028 ShogunGekomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-028 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # [All Turns][Once Per Turn] When a digivolution card of an opponent's Digimon is trashed,
        # this Digimon can't be deleted in battle until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-028 This Digimon can't be deleted by battle")
        effect1.set_effect_description("[All Turns][Once Per Turn] When a digivolution card of an opponent's Digimon is trashed, this Digimon can't be deleted in battle until the end of your opponent's turn.")
        effect1.set_max_count_per_turn(1)
        effect1._is_cannot_be_deleted_by_battle = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False

            target = context.get('target')
            if target is None:
                target = context.get('digimon')
            if target is None:
                target = context.get('opponent_digimon')
            if target is None:
                return False

            player = context.get('player')
            if player is None:
                return False

            owner_getter = getattr(target, 'get_owner', None)
            if callable(owner_getter):
                return owner_getter() != player

            owner = getattr(target, 'owner', None)
            if owner is not None:
                return owner != player

            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Be Deleted By Battle"""
            perm = ctx.get('permanent')
            if perm:
                perm.grant_keyword('_is_cannot_be_deleted_by_battle')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
