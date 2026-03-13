from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_003(CardScript):
    """BT24-003 Tsunomon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnLoseSecurity
        # [Your Turn] [Once Per Turn] When your security stack is removed from, this Digimon may digivolve into a [Shaman] trait Digimon card in the hand with the digivolution cost reduced by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnLoseSecurity)
        effect0.set_effect_name("BT24-003 Digivolve into Shaman trait")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When your security stack is removed from, this Digimon may digivolve into a [Shaman] trait Digimon card in the hand with the digivolution cost reduced by 1.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT24_003_Inherited")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('Shaman' in t for t in traits):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, cost_reduction=1, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
