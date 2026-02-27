from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_001(CardScript):
    """BT14-001 Koromon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnLoseSecurity
        # [Your Turn][Once Per Turn] When a card is removed from your opponent's security stack, <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-001 Draw 1")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When a card is removed from your opponent's security stack, <Draw 1>.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw1_BT14_001")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # Faithful trigger: a card must be removed from the opponent's security stack.
            # Accept common context conventions used by generated scripts.
            if context.get('from_security') is True:
                is_opponent_security = context.get('is_opponent_security')
                if is_opponent_security is None:
                    return True
                return bool(is_opponent_security)

            removed_from = context.get('removed_from')
            if removed_from == 'opponent_security':
                return True

            zone = context.get('zone')
            owner = context.get('owner')
            if zone == 'security' and owner == 'opponent':
                return True

            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
