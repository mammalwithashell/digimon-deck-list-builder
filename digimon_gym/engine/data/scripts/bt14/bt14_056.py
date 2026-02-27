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

        # [On Play] Reveal the top 5 cards of your deck. Add 1 card with the [D-Brigade] or [DigiPolice] trait among them to your hand.
        # Place the rest at either the top or bottom of your deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-056 Reveal top 5 and add [D-Brigade]/[DigiPolice]")
        effect0.set_effect_description("[On Play] Reveal the top 5 cards of your deck. Add 1 card with the [D-Brigade] or [DigiPolice] trait among them to your hand. Place the rest at either the top or bottom of your deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any(('D-Brigade' in t) or ('DigiPolice' in t) for t in traits)

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                # Card text allows top or bottom; engine helper handles this selection path.
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(player, 5, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [All Turns][Once Per Turn] When this Digimon would leave the battle area other than by one of your effects,
        # by deleting 1 of your other Digimon with the [D-Brigade] trait, prevent it from leaving.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-056 Inherited leave prevention via deleting other [D-Brigade]")
        effect1.set_effect_description("[All Turns][Once Per Turn] When this Digimon would leave the battle area other than by one of your effects, by deleting 1 of your other Digimon with the [D-Brigade] trait, prevent it from leaving.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Substitute_BT14_056")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        return effects
