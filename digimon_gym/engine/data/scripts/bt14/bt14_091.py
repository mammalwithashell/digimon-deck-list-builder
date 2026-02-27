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

        # [Main] Trash any 2 digivolution cards from your opponent's Digimon.
        # Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon.
        # If your opponent has no Digimon with as many or more digivolution cards than the chosen Digimon, unsuspend it.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-091 Trash Digivolution Cards, Unsuspend")
        effect0.set_effect_description("[Main] Trash any 2 digivolution cards from your opponent's Digimon. Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon. If your opponent has no Digimon with as many or more digivolution cards than the chosen Digimon, unsuspend it.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            opponent = game.get_opponent(player)
            if opponent:
                remaining = 2
                opp_perms = list(game.get_battle_area_permanents(opponent))
                for perm in opp_perms:
                    if remaining <= 0:
                        break
                    if perm.has_no_digivolution_cards:
                        continue
                    trashed = perm.trash_digivolution_cards(remaining)
                    remaining -= len(trashed)
                    opponent.trash_cards.extend(trashed)

            own_tamers = list(game.get_tamers(player))
            has_joe_kido = any('joe kido' in t.card.name.lower() for t in own_tamers)
            if not has_joe_kido:
                return

            def on_choose(chosen_perm):
                chosen_count = len(chosen_perm.digivolution_cards)
                opp_perms_now = list(game.get_battle_area_permanents(opponent)) if opponent else []
                if all(len(p.digivolution_cards) < chosen_count for p in opp_perms_now):
                    chosen_perm.unsuspend()

            game.effect_select_own_permanent(
                player,
                on_choose,
                filter_fn=lambda p: True,
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
