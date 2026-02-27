from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_040(CardScript):
    """BT8-040 Betsumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may trash 1 card in your hand to treat this Digimon as also having the colors of the trashed card for the turn. Then, if this Digimon has 2 or more colors, <Draw 2>. (Draw 2 cards from your deck.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-040 Trash 1 card from hand to get colors and draw 2")
        effect0.set_effect_description("[When Digivolving] You may trash 1 card in your hand to treat this Digimon as also having the colors of the trashed card for the turn. Then, if this Digimon has 2 or more colors, <Draw 2>. (Draw 2 cards from your deck.)")
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 2, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(2)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
