from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_080(CardScript):
    """BT14-080 Ghoulmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _trash_from_enemy_deck_by_own_trash_count(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player is None:
                return
            enemy = getattr(player, 'enemy', None)
            if enemy is None or not getattr(enemy, 'library_cards', None):
                return

            own_trash = getattr(player, 'trash_cards', [])
            ten_count = len(own_trash) // 10
            trash_count = ten_count * 3
            if trash_count <= 0:
                return

            mill_count = min(trash_count, len(enemy.library_cards))
            trashed = enemy.library_cards[:mill_count]
            enemy.library_cards = enemy.library_cards[mill_count:]
            enemy.trash_cards.extend(trashed)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-080 Trash cards from opponent's deck top")
        effect0.set_effect_description("[When Digivolving][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("TrashDeck_BT14_080")
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_trash_from_enemy_deck_by_own_trash_count)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-080 Trash cards from opponent's deck top")
        effect1.set_effect_description("[When Attacking][Once Per Turn] For every 10 cards in your trash, trash the top 3 cards of your opponent's deck.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashDeck_BT14_080")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_trash_from_enemy_deck_by_own_trash_count)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] If your opponent has 10 or more cards in their trash, this Digimon gains <Security A. +1> for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-080 This Digimon gains Security Attack +1")
        effect2.set_effect_description("[When Attacking][Once Per Turn] If your opponent has 10 or more cards in their trash, this Digimon gains <Security A. +1> for the turn.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("SecurityAttack+1_BT14_080")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            enemy = getattr(player, 'enemy', None) if player else None
            if enemy is None:
                return False
            return len(getattr(enemy, 'trash_cards', [])) >= 10

        def process2(ctx: Dict[str, Any]):
            permanent = ctx.get('permanent')
            if permanent is None:
                return
            if hasattr(permanent, 'add_security_attack_plus_until_end_of_turn'):
                permanent.add_security_attack_plus_until_end_of_turn(1)
            elif hasattr(permanent, 'security_attack_plus'):
                permanent.security_attack_plus += 1

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
