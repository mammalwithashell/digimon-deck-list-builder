from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardColor, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_002(CardScript):
    """BT24-002 Bukamon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] By paying 1 cost, this blue Digimon with the [TS] trait unsuspends.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndTurn)
        effect0.set_effect_name("BT24-002 By paying 1, this digimon may unsuspend")
        effect0.set_effect_description("[End of Your Turn] [Once Per Turn] By paying 1 cost, this blue Digimon with the [TS] trait unsuspends.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("EOYT_BT24_002")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Pay 1 cost, then unsuspend the host permanent if it is blue and has [TS] trait."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return
            # Pay 1 cost
            player.add_memory(-1)
            # Only unsuspend if top card is Blue and has [TS] trait
            if perm.top_card and CardColor.Blue in perm.top_card.card_colors and perm.has_trait('TS'):
                perm.unsuspend()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
