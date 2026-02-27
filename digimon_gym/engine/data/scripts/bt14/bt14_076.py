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
        effect1.set_effect_name("BT14-076 Trash 1 to delete lowest-level Digimon")
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

            def hand_filter(c):
                return True

            def after_trash(_selected):
                # After paying cost, delete lowest-level Digimon on each side.
                # Own side
                own_digimon = [p for p in getattr(player, 'permanents', []) if getattr(p, 'is_digimon', False)]
                if own_digimon:
                    min_lv = min(getattr(p, 'level', 99) for p in own_digimon)
                    own_targets = [p for p in own_digimon if getattr(p, 'level', 99) == min_lv]
                    if own_targets:
                        player.delete_permanent(own_targets[0])

                # Opponent side
                enemy = getattr(player, 'enemy', None)
                if enemy:
                    enemy_digimon = [p for p in getattr(enemy, 'permanents', []) if getattr(p, 'is_digimon', False)]
                    if enemy_digimon:
                        min_lv_e = min(getattr(p, 'level', 99) for p in enemy_digimon)
                        enemy_targets = [p for p in enemy_digimon if getattr(p, 'level', 99) == min_lv_e]
                        if enemy_targets:
                            enemy.delete_permanent(enemy_targets[0])

            game.effect_trash_from_hand(player, 1, hand_filter, after_trash, is_optional=True)

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
                names = getattr(c, 'card_names', [])
                return any('Agumon' in n for n in names)

            has_tai = any(
                getattr(p, 'is_tamer', False) and any('Tai Kamiya' in n for n in getattr(getattr(p, 'card', None), 'card_names', []))
                for p in getattr(player, 'permanents', [])
            )

            def on_played(played_perm):
                if has_tai and played_perm:
                    played_perm.grant_keyword('_is_rush')

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True, on_played=on_played
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
