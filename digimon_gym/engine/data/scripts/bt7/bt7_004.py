from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT7_004(CardScript):
    """BT7-004 Koromon | Digi-Egg Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: is_on_attack (OnAllyAttack path for inherited)
        # [When Attacking] Reveal the top card of your deck, and place it at the top or
        # bottom of your deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT7-004 Reveal top card, place top or bottom")
        effect0.set_effect_description(
            "[When Attacking] Reveal the top card of your deck, and place it at the top "
            "or bottom of your deck."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top card of deck, place at top (selected) or bottom (declined)."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.library_cards:
                return
            # Reveal the top card. If agent selects it → place at top (no change).
            # If agent declines → place at bottom.
            # effect_reveal_and_select: on_selected → card stays (top), on_decline → card
            # goes to bottom via the helper's own decline logic.
            # We override on_selected to put it back at the top explicitly.
            def on_selected(selected, remaining):
                # Agent chose "place at top" — reinsert at position 0
                if selected in player.library_cards:
                    player.library_cards.remove(selected)
                player.library_cards.insert(0, selected)

            game.effect_reveal_and_select(
                player,
                count=1,
                filter_fn=lambda c: True,
                on_selected=on_selected,
                is_optional=True,
                prompt="Place the revealed card at the top (select) or bottom (decline) of your deck.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
