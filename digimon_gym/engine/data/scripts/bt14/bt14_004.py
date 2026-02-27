from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_004(CardScript):
    """BT14-004 Tanemon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Your Turn][Once Per Turn] When one of your effects suspends a Tamer, this Digimon gets +2000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-004 DP +2000")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When one of your effects suspends a Tamer, this Digimon gets +2000 DP for the turn.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DP+2000_BT14_004")
        effect0.dp_modifier = 2000

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # Trigger only when one of your effects suspends a Tamer.
            trigger_type = str(context.get('trigger_type', '')).lower()
            event_type = str(context.get('event_type', '')).lower()
            if 'suspend' not in trigger_type and 'suspend' not in event_type:
                return False

            target = context.get('target') or context.get('suspended_target') or context.get('card')
            if target is None:
                return False

            # Tamer check (supports common shapes used in script contexts)
            is_tamer = False
            kind = getattr(target, 'card_kind', None)
            if kind == 2:
                is_tamer = True
            source = getattr(target, 'source', None)
            if source is not None and getattr(source, 'card_kind', None) == 2:
                is_tamer = True
            if not is_tamer:
                return False

            source_player = context.get('source_player') or context.get('player')
            if source_player is not None and card and card.owner and source_player != card.owner:
                return False

            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +2000"""
            perm = ctx.get('permanent')
            if perm:
                perm.change_dp(2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
