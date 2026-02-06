from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_005(CardScript):
    """Auto-transpiled from DCGO BT14_005.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] By returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, this Digimon gets +2000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-005 Return cards from trash to gain DP +2000")
        effect0.set_effect_description("[When Attacking][Once Per Turn] By returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, this Digimon gets +2000 DP for the turn.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ReturnCards_BT14_005")
        effect0.is_on_attack = True
        effect0.dp_modifier = 2000

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if perm:
                perm.change_dp(2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
