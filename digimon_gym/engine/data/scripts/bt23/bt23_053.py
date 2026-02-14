from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_053(CardScript):
    """BT23-053"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-053 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your Option cards are placed in the battle area, this Digimon may digivolve into a Digimon card with [Cyberdramon] in its name or the [CS] in the hand with the digivolution cost reduced by 2.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-053 Digivolve for reduced cost")
        effect1.set_effect_description("[Your Turn] When any of your Option cards are placed in the battle area, this Digimon may digivolve into a Digimon card with [Cyberdramon] in its name or the [CS] in the hand with the digivolution cost reduced by 2.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Cyberdramon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: dp_modifier
        # DP modifier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-053 DP modifier")
        effect2.set_effect_description("DP modifier")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 1000

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
