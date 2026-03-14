from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_004(CardScript):
    """BT22-004 Wanyamon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn] [Once Per Turn] When effects place [CS] trait Digimon cards in this Digimon's digivolution cards, it may digivolve into a [CS] trait Digimon card in the hand with the digivolution cost reduced by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAddDigivolutionCards)
        effect0.set_effect_name("BT22-004 Digivolve into a [CS] digimon")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When effects place [CS] trait Digimon cards in this Digimon's digivolution cards, it may digivolve into a [CS] trait Digimon card in the hand with the digivolution cost reduced by 1.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT22_004_Digivolve")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the added card has CS trait
            added_card = context.get('added_card')
            if added_card:
                traits = getattr(added_card, 'card_traits', []) or []
                if not any('CS' == t for t in traits):
                    return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve into a CS trait Digimon from hand, cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('CS' == t for t in traits)
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, cost_reduction=1, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
