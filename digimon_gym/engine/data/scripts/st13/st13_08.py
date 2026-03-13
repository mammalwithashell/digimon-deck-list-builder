from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST13_08(CardScript):
    """ST13-08 Chikurimon | Lv.3 Black Rookie Digimon (3000 DP, Cost 3)

    [All Turns] Players can't reduce play costs.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        from ....interfaces.modifiers import ModifierType

        effects = []

        # --- Effect 0: [All Turns] Players can't reduce play costs ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.NoTiming)
        effect0.set_effect_name("ST13-08 Players can't reduce play costs")
        effect0.set_effect_description("[All Turns] Players can't reduce play costs.")
        effect0.is_declarative = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
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
            # Apply CANNOT_REDUCE_COST to both players' permanents
            # This is an aura-style effect; register on self
            game.register_modifier(
                perm, ModifierType.CANNOT_REDUCE_COST,
                value_fn=lambda: True,
                expiry='permanent'
            )
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
