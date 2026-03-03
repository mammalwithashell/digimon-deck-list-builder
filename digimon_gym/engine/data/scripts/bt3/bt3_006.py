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
            game = ctx.get('game')
            if not player:
                return
            # Draw 1
            player.draw_cards(1)
            # Trash 1 from hand
            if not (player.hand_cards and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False,
                prompt="Select a card to trash.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
