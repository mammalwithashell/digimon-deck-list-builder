from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_046(CardScript):
    """EX6-046 DemiDevimon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If your opponent has 5 or fewer cards in their hand, [Draw 1] and trash 1 card in your hand. If your opponent has 7 or more cards in their hand, your opponent trashes 1 card in their hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-046 Draw 1, Trash 1/Opponent trashes card")
        effect0.set_effect_description("[On Deletion] If your opponent has 5 or fewer cards in their hand, [Draw 1] and trash 1 card in your hand. If your opponent has 7 or more cards in their hand, your opponent trashes 1 card in their hand.")
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: dp_modifier
        # DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-046 DP modifier")
        effect1.set_effect_description("DP modifier")
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 1000

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
