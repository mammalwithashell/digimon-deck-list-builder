from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_005(CardScript):
    """EX11-005 Yaamon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] This Digimon may digivolve into a [Dark Dragon] or [Evil Dragon] trait Digimon card in the trash with the digivolution cost reduced by 1. If this effect digivolved, trash 2 cards in your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-005 Digivolve into [Dark Dragon]/[Evil Dragon] trait for 1 less, if so trash 2 from hand.")
        effect0.set_effect_description("[Start of Your Main Phase] This Digimon may digivolve into a [Dark Dragon] or [Evil Dragon] trait Digimon card in the trash with the digivolution cost reduced by 1. If this effect digivolved, trash 2 cards in your hand.")
        effect0.is_inherited_effect = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Dark Dragon' in _t or 'Evil Dragon' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Dark Dragon' in _t or 'Evil Dragon' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
