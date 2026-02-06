from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_064(CardScript):
    """Auto-transpiled from DCGO BT24_064.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-064 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 3 cards of your deck. You may play 1 play cost 7 or lower [Digi] or [SEEKERS] trait card among them without paying the cost. Return the rest to the top or bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-064 Reveal the top 3 cards of deck, play 1")
        effect1.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. You may play 1 play cost 7 or lower [Digi] or [SEEKERS] trait card among them without paying the cost. Return the rest to the top or bottom of the deck.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When any Digimon or Tamers suspend, <De-Digivolve 2> 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-064 De-Digivolve 2")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When any Digimon or Tamers suspend, <De-Digivolve 2> 1 of your opponent's Digimon.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_064_DeDigivolve")

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # De-digivolve opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                removed = target.de_digivolve(1)
                enemy.trash_cards.extend(removed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
