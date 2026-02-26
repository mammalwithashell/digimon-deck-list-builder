from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_006(CardScript):
    """BT13-006 Kapurimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 3 Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-006 Trash 1 card from hand to delete 1 level 3 Digimon")
        effect0.set_effect_description("[On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 3 Digimon.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.is_on_deletion = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = getattr(player, 'enemy', None)
            if enemy is None:
                return

            # Optional effect activation. If chosen, trashing 1 hand card is mandatory.
            if not player.hand_cards:
                return

            def on_trash(selected):
                if selected not in player.hand_cards:
                    return
                player.hand_cards.remove(selected)
                player.trash_cards.append(selected)

                # After paying the cost, delete 1 opponent level 3 Digimon if possible.
                def target_filter(p):
                    if not getattr(p, 'is_digimon', False):
                        return False
                    get_level = getattr(p, 'get_level', None)
                    if callable(get_level):
                        return get_level() == 3
                    return False

                def on_delete(target_perm):
                    if target_perm in enemy.permanents:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player,
                    on_delete,
                    filter_fn=target_filter,
                    is_optional=False,
                )

            game.effect_select_hand_card(
                player,
                lambda c: True,
                on_trash,
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
