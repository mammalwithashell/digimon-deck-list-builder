from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_034(CardScript):
    """BT21-034 Kiwimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-034 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [WG] trait for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_trait = "WG"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('WG' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] (Once Per Turn)  When this Digimon suspends, <Draw 1>
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-034 Draw 1")
        effect1.set_effect_description("[All Turns] (Once Per Turn)  When this Digimon suspends, <Draw 1>")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: jamming
        # Jamming
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-034 Jamming")
        effect2.set_effect_description("Jamming")
        effect2.is_inherited_effect = True
        effect2._is_jamming = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
