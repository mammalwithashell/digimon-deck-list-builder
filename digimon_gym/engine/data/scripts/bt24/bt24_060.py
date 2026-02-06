from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_060(CardScript):
    """Auto-transpiled from DCGO BT24_060.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Reveal the top 3 cards of your deck. This Digimon may digivolve into a [DigiPolice] or [SEEKERS] trait Digimon card among them without paying the cost. Return the rest to the top or bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-060 Reveal the top 3 cards of deck, digivolve into 1")
        effect0.set_effect_description("[When Attacking] Reveal the top 3 cards of your deck. This Digimon may digivolve into a [DigiPolice] or [SEEKERS] trait Digimon card among them without paying the cost. Return the rest to the top or bottom of the deck.")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [All Turns] When Tamer cards are placed in this Digimon's digivolution cards, suspend 1 of your opponent's Digimon. Then, this Digimon may attack your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-060 Suspend 1 opp then this may attack digimon")
        effect1.set_effect_description("[All Turns] When Tamer cards are placed in this Digimon's digivolution cards, suspend 1 of your opponent's Digimon. Then, this Digimon may attack your opponent's Digimon.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your [DigiPolice] or [SEEKERS] trait Digimon would leave the battle area, by playing 1 [DigiPolice] or [SEEKERS] trait Tamer card from this Digimon's digivolution cards without paying the cost, they don't leave.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-060 By playing tamer, card doesn't leave")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When any of your [DigiPolice] or [SEEKERS] trait Digimon would leave the battle area, by playing 1 [DigiPolice] or [SEEKERS] trait Tamer card from this Digimon's digivolution cards without paying the cost, they don't leave.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_060_ESS")

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
