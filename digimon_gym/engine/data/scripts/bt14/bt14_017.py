from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_017(CardScript):
    """BT14-017 Dinorexmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blitz
        # Blitz
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-017 Blitz")
        effect0.set_effect_description("Blitz")
        effect0._is_blitz = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Play Restriction
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-017 Opponent can't play Digimon card with DP 6000 or less")
        effect1.set_effect_description("Play Restriction")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            game = context.get('game')
            player = context.get('player')
            if game is None or player is None:
                return False
            opp = game.get_opponent(player)
            if opp is None:
                return False
            return opp.get_memory() >= 1

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Restriction"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play restriction (CanNotPutFieldClass) — opponent play restrictions
            pass  # descriptive-tagged

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: dp_modifier
        # DP modifier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-017 DP modifier")
        effect2.set_effect_description("DP modifier")
        effect2.dp_modifier = 4000

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            game = context.get('game')
            player = context.get('player')
            if game is None or player is None:
                return False
            opp = game.get_opponent(player)
            if opp is None:
                return False
            return opp.get_memory() >= 1

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
