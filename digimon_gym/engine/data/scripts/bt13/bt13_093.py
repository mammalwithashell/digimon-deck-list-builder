from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_093(CardScript):
    """BT13-093 Omekamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trigger <Draw 1>. (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-093 Draw 1")
        effect0.set_effect_description("[On Play] Trigger <Draw 1>. (Draw 1 card from your deck.)")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

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

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 Digimon card with the [Royal Knight] trait from your hand as the bottom digivolution card of one of your [King Drasil_7D6] in the breeding area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-093 Place 1 card to your [King Drasil_7D6]'s digivolution cards")
        effect1.set_effect_description("[On Deletion] Place 1 Digimon card with the [Royal Knight] trait from your hand as the bottom digivolution card of one of your [King Drasil_7D6] in the breeding area.")
        effect1.is_optional = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
