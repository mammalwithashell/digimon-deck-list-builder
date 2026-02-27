from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_095(CardScript):
    """BT14-095 Poison Ivy"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects: List[ICardEffect] = []

        # [Main] 1 of your opponent's Digimon gains
        # "[All Turns] When this Digimon becomes suspended, lose 2 memory."
        # until the end of your opponent's turn.
        main = ICardEffect()
        main.set_effect_name("BT14-095 Main")
        main.set_effect_description(
            "[Main] 1 of your opponent's Digimon gains \"[All Turns] When this Digimon becomes suspended, lose 2 memory.\" until the end of your opponent's turn."
        )
        main.set_can_use_condition(lambda _ctx: True)

        def main_process(ctx: Dict[str, Any]):
            game = ctx.get("game")
            player = ctx.get("player")
            if game is None or player is None:
                return

            # Select 1 of opponent's Digimon.
            target = None
            if hasattr(game, "select_opponent_digimon"):
                target = game.select_opponent_digimon(player, 1)
                if isinstance(target, list):
                    target = target[0] if target else None
            elif hasattr(game, "choose_opponent_digimon"):
                target = game.choose_opponent_digimon(player)
            elif hasattr(ctx, "get"):
                target = ctx.get("target")

            if target is None:
                return

            # Grant temporary trigger until end of opponent's turn.
            if hasattr(game, "grant_temp_effect"):
                game.grant_temp_effect(
                    target=target,
                    source=card,
                    effect_text="[All Turns] When this Digimon becomes suspended, lose 2 memory.",
                    until="end_of_opponents_turn",
                )
            elif hasattr(target, "add_temp_effect"):
                target.add_temp_effect(
                    "[All Turns] When this Digimon becomes suspended, lose 2 memory.",
                    duration="end_of_opponents_turn",
                    source=card,
                )

        main.set_on_process_callback(main_process)
        effects.append(main)

        # Security: Activate this card's [Main] effects. Then, add this card to your hand.
        security = ICardEffect()
        security.set_effect_name("BT14-095 Security")
        security.set_effect_description(
            "[Security] Activate this card's [Main] effects. Then, add this card to your hand."
        )
        security.set_can_use_condition(lambda _ctx: True)

        def security_process(ctx: Dict[str, Any]):
            # Reuse [Main]
            main_process(ctx)

            game = ctx.get("game")
            player = ctx.get("player")
            if game is None or player is None:
                return

            if hasattr(game, "add_card_to_hand_from_security"):
                game.add_card_to_hand_from_security(player, card)
            elif hasattr(game, "move_card"):
                game.move_card(card, "security", "hand", player)

        security.set_on_process_callback(security_process)
        effects.append(security)

        return effects
