from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_065(CardScript):
    """Auto-transpiled from DCGO BT14_065.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-065 Reveal the top 3 cards of opponent's deck")
        effect0.set_effect_description("[On Play] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal And Select, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice
            # De-digivolve opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                removed = target.de_digivolve(1)
                enemy.trash_cards.extend(removed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-065 Reveal the top 3 cards of opponent's deck")
        effect1.set_effect_description("[When Digivolving] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reveal And Select, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice
            # De-digivolve opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                removed = target.de_digivolve(1)
                enemy.trash_cards.extend(removed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
