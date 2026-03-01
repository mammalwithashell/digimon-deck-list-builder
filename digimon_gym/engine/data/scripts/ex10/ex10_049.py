from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_049(CardScript):
    """EX10-049 SkullSatamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-049 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If your opponent has 10 or fewer cards in their trash, trash the top 3 cards of both players' decks. Then, delete 1 of your opponent's level 3 or lower Digimon. If your opponent has 10 or more cards in their trash, add 2 to this effect's level maximum.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX10-049 If opponent has 10 or more trash cards, trash top 3 of both player decks. then delete 1 level 3 or lower digimon, add 2 levels if enemy trash is 10 or more")
        effect1.set_effect_description("[When Digivolving] If your opponent has 10 or fewer cards in their trash, trash the top 3 cards of both players' decks. Then, delete 1 of your opponent's level 3 or lower Digimon. If your opponent has 10 or more cards in their trash, add 2 to this effect's level maximum.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Mill 3 cards from own deck
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If your opponent has 10 or fewer cards in their trash, trash the top 3 cards of both players' decks. Then, delete 1 of your opponent's level 3 or lower Digimon. If your opponent has 10 or more cards in their trash, add 2 to this effect's level maximum.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX10-049 If opponent has 10 or more trash cards, trash top 3 of both player decks. then delete 1 level 3 or lower digimon, add 2 levels if enemy trash is 10 or more")
        effect2.set_effect_description("[On Deletion] If your opponent has 10 or fewer cards in their trash, trash the top 3 cards of both players' decks. Then, delete 1 of your opponent's level 3 or lower Digimon. If your opponent has 10 or more cards in their trash, add 2 to this effect's level maximum.")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Mill 3 cards from own deck
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] This Digimon gains <Security A. +1> for the turn. (This Digimon checks 1 additional security card.) If your opponent has 10 or fewer cards in their trash, instead trash the top 2 cards of both players' decks.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnAllyAttack)
        effect3.set_effect_name("EX10-049 If opponent has 10 or less trash cards, both player trash 2 cards from top deck, otherwise Sec +1")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] This Digimon gains <Security A. +1> for the turn. (This Digimon checks 1 additional security card.) If your opponent has 10 or fewer cards in their trash, instead trash the top 2 cards of both players' decks.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX10_049_WA")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Mill 2 cards from own deck
            if player and player.library_cards:
                mill_count = min(2, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
