from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_068(CardScript):
    """BT14-068 Brigadramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] Delete up to 7 play cost's total worth of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-068 Delete Digimon")
        effect0.set_effect_description("[When Digivolving] Delete up to 7 play cost's total worth of your opponent's Digimon.")
        effect0.is_when_digivolving = True

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

            total = {'cost': 0}

            def target_filter(p):
                if not getattr(p, 'is_digimon', False):
                    return False
                play_cost = getattr(getattr(p, 'card', None), 'play_cost', None)
                if play_cost is None:
                    return False
                return total['cost'] + play_cost <= 7

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if not enemy:
                    return
                play_cost = getattr(getattr(target_perm, 'card', None), 'play_cost', 0) or 0
                total['cost'] += play_cost
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Opponent's Turn] All of your Digimon with the [D-Brigade] trait gain <Blocker>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-068 D-Brigade Blocker Aura")
        effect1.set_effect_description("[Opponent's Turn] All of your Digimon with the [D-Brigade] trait gain <Blocker>.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            return not owner.is_my_turn

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            permanent = ctx.get('permanent')
            if not (player and permanent):
                return
            if not getattr(permanent, 'is_digimon', False):
                return
            traits = getattr(getattr(permanent, 'card', None), 'type_eng', []) or []
            if 'D-Brigade' in traits:
                permanent._is_blocker = True

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [End of Your Turn] [Once Per Turn] Reveal top 3. You may play up to total 7 play cost
        # of cards with [D-Brigade] or [DigiPolice] among them without paying costs. Trash the rest.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-068 Reveal top 3 and play")
        effect2.set_effect_description("[End of Your Turn][Once Per Turn] Reveal the top 3 cards of your deck. You may play up to 7 play cost's total worth of cards with the [D-Brigade] or [DigiPolice] trait among them without paying the costs. Trash the rest.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Reveal_BT14_068")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            selected_cards = []
            remaining_cards = []

            def reveal_filter(c):
                return True

            def on_revealed(selected, remaining):
                nonlocal selected_cards, remaining_cards
                selected_cards = selected or []
                remaining_cards = remaining or []

            game.effect_reveal_and_select(player, 3, reveal_filter, on_revealed, is_optional=False)

            total_play_cost = 0
            to_trash = list(selected_cards)

            for c in list(selected_cards):
                traits = getattr(c, 'type_eng', []) or []
                play_cost = getattr(c, 'play_cost', None)
                if play_cost is None:
                    continue
                if ('D-Brigade' in traits or 'DigiPolice' in traits) and total_play_cost + play_cost <= 7:
                    game.effect_play_from_reveal(player, c, free=True)
                    total_play_cost += play_cost
                    if c in to_trash:
                        to_trash.remove(c)

            for c in to_trash + list(remaining_cards):
                player.trash_cards.append(c)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
