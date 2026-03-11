from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_004(CardScript):
    """BT24-004 Wanyamon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When any of your [Iliad] trait Digimon are played, <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT24-004 Draw 1")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When any of your [Iliad] trait Digimon are played, <Draw 1>.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT24_004_Draw1")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The entering permanent must be owned by this card's owner and have [Iliad] trait
            entering_perm = context.get('permanent')
            if entering_perm is None:
                return False
            # Must be on the owner's side (not the opponent's)
            perm_owner = getattr(entering_perm, 'owner', None)
            if perm_owner is None or perm_owner is not (card.owner if card else None):
                return False
            top = getattr(entering_perm, 'top_card', None)
            if top is None:
                return False
            traits = getattr(top, 'card_traits', []) or []
            if not any('Iliad' in t for t in traits):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
