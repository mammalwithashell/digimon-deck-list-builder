from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_057(CardScript):
    """EX8-057 DemiDevimon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.2 with [NSo] trait for cost 0
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-057 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "NSo"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('NSo' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ---------------------------------------------------------------
        # On Play: Reveal top 3 cards of deck.
        # Add 1 [NSo] trait card and 1 [Fallen Angel] trait card to hand.
        # Return the rest to the bottom of the deck.
        # ---------------------------------------------------------------
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX8-057 Reveal the top 3 cards of deck")
        effect1.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [NSo] trait and 1 card with the [Fallen Angel] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reveal top 3, select 1 NSo + 1 Fallen Angel to hand, rest to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.library_cards:
                return

            def nso_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('NSo' in tr for tr in traits)

            def fallen_angel_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Fallen Angel' in tr for tr in traits)

            # Use multi-pass reveal: pass 1 = NSo to hand, pass 2 = Fallen Angel to hand
            game.effect_reveal_and_select_multi(
                player, count=3,
                passes=[
                    (nso_filter, 'hand'),
                    (fallen_angel_filter, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ---------------------------------------------------------------
        # ESS When Attacking: [Once Per Turn] Draw 1 and trash 1 from hand
        # ---------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX8-057 Draw 1 and trash 1 card from hand")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] <Draw 1> and trash 1 card in your hand.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Draw_EX8_057")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1, Trash 1 from hand"""
            player = ctx.get('player')
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
