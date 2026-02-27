from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_029(CardScript):
    """BT14-029 Plesiomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-029 Trash digivolution cards")
        effect0.set_effect_description("[When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon.")
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            return not (card and card.permanent_of_this_card() is None)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(target_perm):
                return not target_perm.has_no_digivolution_cards

            def on_trash(target_perm):
                target_perm.trash_digivolution_cards(3)

            game.effect_select_opponent_permanent(
                player, on_trash, filter_fn=target_filter, is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Attacking][Once Per Turn] If your opponent has no Digimon with as many or more
        # digivolution cards as this Digimon, unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-029 Unsuspend this Digimon")
        effect1.set_effect_description("[When Attacking][Once Per Turn] If your opponent has no Digimon with as many or more digivolution cards as this Digimon, unsuspend this Digimon.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT14_029")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            perm = context.get('permanent')
            game = context.get('game')
            if not (perm and game):
                return False

            my_count = len(getattr(perm, 'digivolution_cards', []) or [])
            for opp_perm in game.get_opponent_player(perm.controller).battle_area:
                opp_count = len(getattr(opp_perm, 'digivolution_cards', []) or [])
                if opp_count >= my_count:
                    return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            perm = ctx.get('permanent')
            if perm:
                perm.unsuspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
