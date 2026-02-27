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

        # [When Digivolving] By trashing 1 card in your hand, delete 1 of your Digimon
        # with the lowest level and 1 of your opponent's Digimon with the lowest level.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-076 Trash 1 card from hand, then delete lowest level Digimons")
        effect1.set_effect_description("[When Digivolving] By trashing 1 card in your hand, delete 1 of your Digimon with the lowest level and 1 of your opponent's Digimon with the lowest level.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            if not getattr(player, 'hand_cards', None):
                return

            trashed_flag = {'ok': False}

            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    trashed_flag['ok'] = True

            game.effect_select_hand_card(player, hand_filter, on_trashed, is_optional=False)
            if not trashed_flag['ok']:
                return

            def level_value(p):
                card_obj = getattr(p, 'card', None)
                return getattr(card_obj, 'level', None)

            my_digimon = [p for p in getattr(player, 'battle_area', []) if getattr(p, 'is_digimon', False)]
            enemy = getattr(player, 'enemy', None)
            enemy_digimon = [p for p in getattr(enemy, 'battle_area', []) if getattr(p, 'is_digimon', False)] if enemy else []

            my_levels = [lv for lv in (level_value(p) for p in my_digimon) if isinstance(lv, int)]
            enemy_levels = [lv for lv in (level_value(p) for p in enemy_digimon) if isinstance(lv, int)]

            if my_levels:
                min_my = min(my_levels)
                my_targets = [p for p in my_digimon if level_value(p) == min_my]

                def my_filter(p):
                    return p in my_targets

                def on_delete_my(target_perm):
                    if target_perm:
                        player.delete_permanent(target_perm)

                game.effect_select_permanent(player, on_delete_my, filter_fn=my_filter, is_optional=False)

            if enemy and enemy_levels:
                min_enemy = min(enemy_levels)
                enemy_targets = [p for p in enemy_digimon if level_value(p) == min_enemy]

                def enemy_filter(p):
                    return p in enemy_targets

                def on_delete_enemy(target_perm):
                    if target_perm:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(player, on_delete_enemy, filter_fn=enemy_filter, is_optional=False)

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

            def has_tai_kamiya_tamer() -> bool:
                for p in getattr(player, 'battle_area', []):
                    card_obj = getattr(p, 'card', None)
                    if not card_obj:
                        continue
                    if getattr(card_obj, 'card_kind', None) != 1:
                        continue
                    names = getattr(card_obj, 'card_names', []) or []
                    if any('Tai Kamiya' in n for n in names):
                        return True
                return False

            give_rush = has_tai_kamiya_tamer()

            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Agumon' in n for n in names)

            def on_played(played_perm):
                if give_rush and played_perm:
                    played_perm.grant_keyword('_is_rush')

            game.effect_play_from_zone(
                player,
                'trash',
                play_filter,
                free=True,
                is_optional=True,
                on_played=on_played
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
