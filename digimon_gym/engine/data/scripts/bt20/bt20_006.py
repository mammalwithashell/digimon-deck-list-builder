from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_006(CardScript):
    """BT20-006 DemiMeramon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may return 1 Digimon card with the [Ghost] trait from your trash to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-006 Return 1 card from trash to hand")
        effect0.set_effect_description("[On Deletion] You may return 1 Digimon card with the [Ghost] trait from your trash to the hand.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
