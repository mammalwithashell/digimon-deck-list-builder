from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_006(CardScript):
    """BT3-006 DemiMeramon | Lv.2 Purple Digi-Egg | Flame"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [On Deletion] Draw 1, then trash 1 from hand ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT3-006 Inherited: On Deletion Draw 1, trash 1")
        effect0.set_effect_description(
            "Inherited: [On Deletion] <Draw 1> Then, trash 1 card in your hand."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_deletion = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, then trash 1 from hand"""
            player = ctx.get('player')
            if not player:
                return
            # Draw 1
            player.draw_cards(1)
            # Trash 1 from hand
            if player.hand_cards:
                trashed = player.hand_cards.pop()
                player.trash_cards.append(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
