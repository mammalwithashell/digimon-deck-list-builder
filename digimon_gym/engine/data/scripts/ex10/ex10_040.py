from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_040(CardScript):
    """EX10-040 DemiDevimon | Lv.3

    [Start of Your Main Phase] If your opponent has 10 or fewer cards in
        their trash, trash the top 2 cards of both players' decks. Then,
        if they have 10 or more cards in their trash, gain 1 memory.
    Inherited: [When Attacking] [Once Per Turn] Trash the top card of both
        players' decks.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Main Phase] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX10-040 Trash top 2 both decks, conditional memory")
        effect0.set_effect_description(
            "[Start of Your Main Phase] If your opponent has 10 or fewer cards "
            "in their trash, trash the top 2 cards of both players' decks. Then, "
            "if they have 10 or more cards in their trash, gain 1 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Only mill if opponent has 10 or fewer cards in trash
            if len(enemy.trash_cards) <= 10:
                # Trash top 2 of both players' decks
                for p in [player, enemy]:
                    count = min(2, len(p.library_cards))
                    for _ in range(count):
                        if p.library_cards:
                            top = p.library_cards.pop(0)
                            p.trash_cards.append(top)

            # Then, if opponent has 10 or more in trash, gain 1 memory
            if len(enemy.trash_cards) >= 10:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [When Attacking] [Once Per Turn]
        #     Trash top card of both players' decks ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("EX10-040 Trash top card from both decks")
        effect1.set_effect_description(
            "[When Attacking] [Once Per Turn] Trash the top card of both "
            "players' decks.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashTopCard_EX10_040")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Trash top 1 card of both players' decks
            for p in [player, enemy]:
                if p.library_cards:
                    top = p.library_cards.pop(0)
                    p.trash_cards.append(top)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
