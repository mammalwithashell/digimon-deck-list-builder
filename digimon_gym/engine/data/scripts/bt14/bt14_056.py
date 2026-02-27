from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_056(CardScript):
    """BT14-056 Commandramon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 5 cards of your deck. Add 1 card with the [D-Brigade] or [DigiPolice] trait among them to the hand. Return the rest to the top or bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-056 Reveal the top 5 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 5 cards of your deck. Add 1 card with the [D-Brigade] or [DigiPolice] trait among them to the hand. Return the rest to the top or bottom of the deck.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal and select 1 matching trait card to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                return any(
                    ('D-Brigade' in t) or ('DigiPolice' in t)
                    for t in (getattr(c, 'card_traits', []) or [])
                )

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                # Engine handles placing remaining revealed cards to top or bottom.

            game.effect_reveal_and_select(
                player, 5, reveal_filter, on_revealed, is_optional=True
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns][Once Per Turn] When this Digimon would leave the battle area other than by one of your effects, by deleting 1 of your other Digimon with the [D-Brigade] trait, prevent it from leaving.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-056 Delete your 1 other Digimon to prevent this Digimon from leaving Battle Area")
        effect1.set_effect_description("[All Turns][Once Per Turn] When this Digimon would leave the battle area other than by one of your effects, by deleting 1 of your other Digimon with the [D-Brigade] trait, prevent it from leaving.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Substitute_BT14_056")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
