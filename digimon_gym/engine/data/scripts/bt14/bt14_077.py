from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_077(CardScript):
    """Auto-transpiled from DCGO BT14_077.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 2 cards of both players' decks.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-077 Both players trash the top 2 cards of their decks")
        effect0.set_effect_description("[On Play] Trash the top 2 cards of both players' decks.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash the top 2 cards of both players' decks.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-077 Both players trash the top 2 cards of their decks")
        effect1.set_effect_description("[When Digivolving] Trash the top 2 cards of both players' decks.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDiscardLibrary
        # [Your Turn][Once Per Turn] When a card in your opponent's deck is trashed, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-077 Memory +1")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When a card in your opponent's deck is trashed, gain 1 memory.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Memory+_BT14_077")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
