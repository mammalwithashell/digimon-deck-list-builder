from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_054(CardScript):
    """BT22-054 Hagurumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-054 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.2 for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS] trait in this Digimon's digivolution cards, 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-054 Give 1 digimon -3k DP")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS] trait in this Digimon's digivolution cards, 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT22_054_OnAddDigivolutionCards")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] [Once Per Turn] By placing this [CS] trait Digimon's top stacked card as its bottom digivolution card, <Draw 1> (Draw 1 card from your deck).
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-054 Place the top card of this Digimon at the bottom of digivolution cards to Draw 1")
        effect2.set_effect_description("[Main] [Once Per Turn] By placing this [CS] trait Digimon's top stacked card as its bottom digivolution card, <Draw 1> (Draw 1 card from your deck).")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ReturnDigivolutionCards_BT22_054")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 1):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
