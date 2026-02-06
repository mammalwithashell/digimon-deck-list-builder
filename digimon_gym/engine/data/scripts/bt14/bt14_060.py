from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_060(CardScript):
    """Auto-transpiled from DCGO BT14_060.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-060 Also treated as [Commandramon]")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Reveal the top 3 cards of your deck. You may play 1 card with the [D-Brigade] or [DigiPolice] trait and a play cost of 3 or less among them without paying the cost. Return the rest to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-060 Reveal the top 3 cards of deck")
        effect1.set_effect_description("[When Attacking] Reveal the top 3 cards of your deck. You may play 1 card with the [D-Brigade] or [DigiPolice] trait and a play cost of 3 or less among them without paying the cost. Return the rest to the bottom of the deck.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns][Once Per Turn] When this Digimon would leave the battle area other than by one of your effects, by deleting 1 of your other Digimon with the [D-Brigade] trait, prevent it from leaving.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-060 Delete your 1 other Digimon to prevent this Digimon from leaving Battle Area")
        effect2.set_effect_description("[All Turns][Once Per Turn] When this Digimon would leave the battle area other than by one of your effects, by deleting 1 of your other Digimon with the [D-Brigade] trait, prevent it from leaving.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Substitute_BT14_060")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
