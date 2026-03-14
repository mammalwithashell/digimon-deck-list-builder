from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT7_069(CardScript):
    """BT7-069 Eyesmon: Scatter Mode | Lv.4
    [On Deletion] <Draw 3>. Then, trash 2 cards in your hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Deletion] Draw 3, then trash 2 from hand
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT7-069 On Deletion: Draw 3 trash 2")
        effect0.set_effect_description(
            "[On Deletion] Draw 3. Then, trash 2 cards in your hand."
        )
        effect0.is_on_deletion = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            player.draw_cards(3)
            if not game:
                return

            def hand_filter(c):
                return True

            def on_trashed_second(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                if player.hand_cards:
                    game.effect_select_hand_card(
                        player, hand_filter, on_trashed_second,
                        is_optional=False,
                        prompt="Select a second card to trash.")

            if player.hand_cards:
                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=False,
                    prompt="Select a card to trash.")
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
