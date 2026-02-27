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
        effect1.set_effect_name("BT14-068 Opponent Turn D-Brigade Blocker")
        effect1.set_effect_description("[Opponent's Turn] All of your Digimon with the [D-Brigade] trait gain ＜Blocker＞.")

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner):
                return False
            owner = card.owner
            if owner.is_my_turn:
                return False
            perm = context.get('permanent')
            if not perm or not getattr(perm, 'is_digimon', False):
                return False
            if getattr(perm, 'owner', None) != owner:
                return False
            traits = getattr(getattr(perm, 'card', None), 'type_eng', []) or []
            return 'D-Brigade' in traits

        effect1.set_can_use_condition(condition1)
        effect1._is_blocker = True
        effects.append(effect1)

        # [End of Your Turn] [Once Per Turn] Reveal top 3; may play up to total 7 cost
        # of D-Brigade/DigiPolice cards among them for free. Trash the rest.
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

            remaining_cost = 7
            to_play = []
            rest = []

            for c in revealed:
                traits = getattr(c, 'type_eng', []) or []
                play_cost = getattr(c, 'play_cost', None)
                valid_trait = ('D-Brigade' in traits) or ('DigiPolice' in traits)
                if valid_trait and isinstance(play_cost, int) and play_cost <= remaining_cost:
                    to_play.append(c)
                    remaining_cost -= play_cost
                else:
                    rest.append(c)

            for c in to_play:
                player.hand_cards.append(c)

            def play_filter(c):
                return c in to_play

            player_ctx = {'player': player, 'game': ctx.get('game'), 'permanent': ctx.get('permanent')}
            game = ctx.get('game')
            if game and to_play:
                for _ in range(len(to_play)):
                    game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=True)

            player.trash_cards.extend(rest)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
