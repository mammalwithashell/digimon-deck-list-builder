from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_160(CardScript):
    """P-160 Tyrannomon (X Antibody) | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-160 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 with [X Antibody] trait for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "X Antibody"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('X Antibody' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("P-160 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If a card with [Tyrannomon] in its name or [X Antibody] is in this Digimon's digivolution cards, this Digimon may digivolve into a Digimon card with [Tyrannomon] in its name or the [Dinosaur] trait in the hand with the digivolution cost reduced by 1.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-160 This Digimon digivolves")
        effect2.set_effect_description("[When Attacking] If a card with [Tyrannomon] in its name or [X Antibody] is in this Digimon's digivolution cards, this Digimon may digivolve into a Digimon card with [Tyrannomon] in its name or the [Dinosaur] trait in the hand with the digivolution cost reduced by 1.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Tyrannomon' in _n for _n in getattr(c, 'card_names', [])) or any('Dinosaur' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
