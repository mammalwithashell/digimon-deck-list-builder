from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_007(CardScript):
    """BT14-007 Agumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-007 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its name, this Digimon may digivolve into [Greymon] in your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-007 This Digimon digivolves")
        effect1.set_effect_description("[Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its name, this Digimon may digivolve into [Greymon] in your hand without paying the cost.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = context.get('player')
            if not player:
                return False
            return player.has_tamer_with_name("Tai Kamiya")

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                return any('Greymon' in _n for _n in getattr(c, 'card_names', []))

            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: dp_modifier
        # DP modifier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-007 DP modifier")
        effect2.set_effect_description("DP modifier")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            names = getattr(perm, 'card_names', [])
            return any(('Greymon' in n) or ('Omnimon' in n) for n in names)

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
