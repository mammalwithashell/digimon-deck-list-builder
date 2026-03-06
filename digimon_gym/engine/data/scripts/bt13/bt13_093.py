from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_093(CardScript):
    """BT13-093 Omekamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <Draw 1> (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-093 Draw 1")
        effect0.set_effect_description("[On Play] <Draw 1> (Draw 1 card from your deck.)")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play -- validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 Digimon card with the [Royal Knight] trait from
        # your hand under a [King Drasil_7D6] in the breeding area as its bottom
        # digivolution card.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT13-093 Place Royal Knight under King Drasil_7D6")
        effect1.set_effect_description(
            "[On Deletion] Place 1 Digimon card with the [Royal Knight] trait from "
            "your hand under a [King Drasil_7D6] in the breeding area as its bottom "
            "digivolution card."
        )
        effect1.is_on_deletion = True
        # Card text has no "may" -- this is mandatory (is_optional defaults to False)

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Must have a Royal Knight Digimon in hand
            if not (card and card.owner):
                return False
            player = card.owner
            has_royal_knight_in_hand = False
            for hand_card in player.hand_cards:
                if not getattr(hand_card, 'is_digimon', False):
                    continue
                traits = getattr(hand_card, 'card_traits', []) or []
                if any("Royal Knight" in trait for trait in traits):
                    has_royal_knight_in_hand = True
                    break
            if not has_royal_knight_in_hand:
                return False
            # Must have a King Drasil_7D6 in the breeding area
            breeding = player.breeding_area
            if breeding is None:
                return False
            if not breeding.contains_card_name('King Drasil_7D6'):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Place Royal Knight from hand under King Drasil_7D6 in breeding."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            breeding = player.breeding_area
            if breeding is None or not breeding.contains_card_name('King Drasil_7D6'):
                return

            def hand_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any("Royal Knight" in trait for trait in traits)

            def on_select(selected_card):
                # Remove the card from hand
                if selected_card in player.hand_cards:
                    player.hand_cards.remove(selected_card)
                # Insert at index 0 (bottom of the digivolution stack)
                breeding_perm = player.breeding_area
                if breeding_perm is not None:
                    breeding_perm.card_sources.insert(0, selected_card)

            game.effect_select_hand_card(
                player, hand_filter, on_select, is_optional=False,
                prompt="Select a Royal Knight Digimon from your hand to place under King Drasil_7D6."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
