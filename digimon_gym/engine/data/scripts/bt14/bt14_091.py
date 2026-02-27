from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_091(CardScript):
    """BT14-091 Wave of Reliability"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Trash 2 digivolution cards from your opponent's Digimon.
        # Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon.
        # If your opponent has no Digimon with as many or more digivolution cards as that Digimon, unsuspend it.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-091 Trash Digivolution Cards, Unsuspend")
        effect0.set_effect_description("[Main] Trash 2 digivolution cards from your opponent's Digimon. Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon. If your opponent has no Digimon with as many or more digivolution cards as that Digimon, unsuspend it.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Trash exactly 2 total digivolution cards from opponent Digimon.
            remaining = 2
            while remaining > 0:
                trashed_any = {'done': False}

                def can_target_opp(p):
                    return not p.has_no_digivolution_cards

                def on_trash_one(target_perm):
                    trashed = target_perm.trash_digivolution_cards(1)
                    if trashed:
                        player.trash_cards.extend(trashed)
                        trashed_any['done'] = True

                game.effect_select_opponent_permanent(
                    player, on_trash_one, filter_fn=can_target_opp, is_optional=False
                )

                if not trashed_any['done']:
                    break
                remaining -= 1

            # Check for [Joe Kido] Tamer.
            has_joe_kido = False
            for tamer in getattr(player, 'tamers', []):
                name = getattr(tamer, 'name', '')
                if 'Joe Kido' in name:
                    has_joe_kido = True
                    break
            if not has_joe_kido:
                return

            def on_choose_own(chosen_perm):
                chosen_count = len(getattr(chosen_perm, 'digivolution_cards', []) or [])
                opponent_has_as_many_or_more = False
                for opp in game.get_opponent(player).battle_area:
                    opp_count = len(getattr(opp, 'digivolution_cards', []) or [])
                    if opp_count >= chosen_count:
                        opponent_has_as_many_or_more = True
                        break
                if not opponent_has_as_many_or_more:
                    chosen_perm.unsuspend()

            game.effect_select_own_permanent(
                player, on_choose_own, filter_fn=lambda p: True, is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
