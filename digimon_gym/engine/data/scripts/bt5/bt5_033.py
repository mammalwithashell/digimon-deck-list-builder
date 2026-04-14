from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_033(CardScript):
    """BT5-033 Cutemon | Lv.3 Yellow Digimon (DP 3000, Cost 3)

    [Opponent's Turn] Your opponent can't reduce digivolution costs.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        from ....interfaces.modifiers import ModifierType

        effects = []

        # --- Effect 0: [Opponent's Turn] Opponent can't reduce evo costs ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.NoTiming)
        effect0.set_effect_name("BT5-033 Opponent can't reduce evo costs")
        effect0.set_effect_description(
            "[Opponent's Turn] Your opponent can't reduce digivolution costs."
        )
        effect0.is_declarative = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Active on opponent's turn only
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            perm = card.permanent_of_this_card()
            if not perm:
                return
            # Register CANNOT_REDUCE_COST modifier — active only on opponent's turn
            # The condition checks that the card owner's turn is NOT active
            # (i.e., it's the opponent's turn)
            game.register_modifier(
                perm, ModifierType.CANNOT_REDUCE_COST,
                condition=lambda target, context, c=card: (
                    c.permanent_of_this_card() is not None
                    and c.owner is not None
                    and not c.owner.is_my_turn
                ),
                value_fn=lambda: True,
                expiry='permanent',
            )
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
