from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_022(CardScript):
    """BT14-022 Gesomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Trash any 1 digivolution card of 1 of your opponent's Digimon. Then, return 1 of your opponent's level 5 or lower Digimon with no digivolution cards to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-022 Trash digivolution cards and return 1 Digimon to hand")
        effect0.set_effect_description("[When Attacking] Trash any 1 digivolution card of 1 of your opponent's Digimon. Then, return 1 of your opponent's level 5 or lower Digimon with no digivolution cards to the hand.")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash 1 opponent digivolution card, then bounce eligible opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = player.enemy
            if enemy:
                # Step 1: Trash 1 digivolution card from 1 opponent Digimon.
                def can_trash_target(p):
                    return not p.has_no_digivolution_cards

                def on_trash_target(target_perm):
                    trashed = target_perm.trash_digivolution_cards(1)
                    if player and trashed:
                        player.trash_cards.extend(trashed)

                game.effect_select_opponent_permanent(
                    player, on_trash_target, filter_fn=can_trash_target, is_optional=False
                )

                # Step 2: Return 1 opponent level 5 or lower Digimon with no digivolution cards.
                def can_bounce_target(p):
                    if p.level is None or p.level > 5:
                        return False
                    return p.has_no_digivolution_cards

                def on_bounce_target(target_perm):
                    enemy.bounce_permanent_to_hand(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_bounce_target, filter_fn=can_bounce_target, is_optional=False
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
