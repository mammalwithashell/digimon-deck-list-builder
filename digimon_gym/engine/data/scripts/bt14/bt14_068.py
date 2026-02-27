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

            remaining = {'cost': 7}

            def target_filter(p):
                if not getattr(p, 'is_digimon', False):
                    return False
                play_cost = getattr(getattr(p, 'card', None), 'play_cost', None)
                if play_cost is None:
                    return False
                return play_cost <= remaining['cost']

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if not enemy:
                    return
                play_cost = getattr(getattr(target_perm, 'card', None), 'play_cost', 0) or 0
                enemy.delete_permanent(target_perm)
                remaining['cost'] -= play_cost

            while remaining['cost'] > 0:
                before = remaining['cost']
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=True)
                if remaining['cost'] == before:
                    break

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Opponent's Turn] All of your Digimon with the [D-Brigade] trait gain <Blocker>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-068 D-Brigade Blocker Aura")
        effect1.set_effect_description("[Opponent's Turn] All of your Digimon with the [D-Brigade] trait gain <Blocker>.")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner or owner.is_my_turn:
                return False
            permanent = context.get('permanent')
            if not permanent or not getattr(permanent, 'is_digimon', False):
                return False
            traits = getattr(getattr(permanent, 'card', None), 'type_eng', []) or []
            return 'D-Brigade' in traits

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [End of Your Turn] [Once Per Turn] Reveal top 3; play eligible cards up to total play cost 7; trash the rest.
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
            if not player:
                return

            revealed = []
            for _ in range(3):
                if not player.library_cards:
                    break
                revealed.append(player.library_cards.pop(0))

            if not revealed:
                return

            def has_required_trait(c):
                traits = getattr(c, 'type_eng', []) or []
                return ('D-Brigade' in traits) or ('DigiPolice' in traits)

            remaining_cost = 7
            to_play = []
            rest = []
            for c in revealed:
                pc = getattr(c, 'play_cost', None)
                if pc is None:
                    rest.append(c)
                    continue
                if has_required_trait(c) and pc <= remaining_cost:
                    to_play.append(c)
                    remaining_cost -= pc
                else:
                    rest.append(c)

            for c in to_play:
                player.hand_cards.append(c)

            for c in rest:
                player.trash_cards.append(c)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
