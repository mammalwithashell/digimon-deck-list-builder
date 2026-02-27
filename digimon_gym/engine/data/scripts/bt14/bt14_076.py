from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_076(CardScript):
    """BT14-076 SkullGreymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-076 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Digivolving] By trashing 1 card in your hand, delete 1 of your Digimon with the lowest level
        # and 1 of your opponent's Digimon with the lowest level.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-076 Trash 1 card from hand to delete lowest-level Digimon")
        effect1.set_effect_description("[When Digivolving] By trashing 1 card in your hand, delete 1 of your Digimon with the lowest level and 1 of your opponent's Digimon with the lowest level.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

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

            # Cost: trash 1 card from hand first. If not paid, do nothing.
            paid = {'ok': False}

            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    paid['ok'] = True

            game.effect_select_hand_card(player, hand_filter, on_trashed, is_optional=True)
            if not paid['ok']:
                return

            def _level_of(p):
                try:
                    return int(getattr(getattr(p, 'card_data', None), 'level', None) or 99)
                except Exception:
                    return 99

            def _lowest_level_filter(owner):
                candidates = [p for p in getattr(owner, 'battle_area', []) if getattr(p, 'is_digimon', False)]
                if not candidates:
                    return None
                min_lv = min(_level_of(p) for p in candidates)
                return lambda p: getattr(p, 'is_digimon', False) and _level_of(p) == min_lv

            my_filter = _lowest_level_filter(player)
            if my_filter is not None:
                def on_delete_my(target_perm):
                    player.delete_permanent(target_perm)
                game.effect_select_permanent(player, on_delete_my, filter_fn=my_filter, is_optional=False)

            enemy = getattr(player, 'enemy', None)
            if enemy is not None:
                opp_filter = _lowest_level_filter(enemy)
                if opp_filter is not None:
                    def on_delete_opp(target_perm):
                        enemy.delete_permanent(target_perm)
                    game.effect_select_opponent_permanent(player, on_delete_opp, filter_fn=opp_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [On Deletion] You may play 1 [Agumon] from your trash without paying the cost.
        # If you have a Tamer with [Tai Kamiya] in its name, that Digimon gains <Rush> for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-076 Play 1 [Agumon] from trash")
        effect2.set_effect_description("[On Deletion] You may play 1 [Agumon] from your trash without paying the cost. If you have a Tamer with [Tai Kamiya] in its name, that Digimon gains <Rush> for the turn.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                return any('Agumon' in n for n in getattr(c, 'card_names', []))

            had_tai = any(
                any('Tai Kamiya' in n for n in getattr(getattr(p, 'card_data', None), 'card_names', []))
                for p in getattr(player, 'battle_area', [])
                if getattr(p, 'is_tamer', False)
            )

            played_holder = {'perm': None}

            def on_played(new_perm):
                played_holder['perm'] = new_perm

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True, on_played=on_played
            )

            if had_tai and played_holder['perm'] is not None:
                played_holder['perm'].grant_keyword('_is_rush')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
