from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_049(CardScript):
    """BT22-049 Vegiemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-049 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 for cost 2
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End Of Your Turn] [Once Per Turn] By placing 3 Digimon cards with the [Ver.2] trait from your trash face down as this Digimon's bottom digivolution cards, it may digivolve into a Digimon card with the [Ver.2] trait in the hand or trash.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("BT22-049 Digivolve into a [Ver.2]")
        effect1.set_effect_description("[End Of Your Turn] [Once Per Turn] By placing 3 Digimon cards with the [Ver.2] trait from your trash face down as this Digimon's bottom digivolution cards, it may digivolve into a Digimon card with the [Ver.2] trait in the hand or trash.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Digivolve_BT22-049")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Place 3 Ver.2 from trash, digivolve into Ver.2"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # Cost: Place 3 Ver.2 Digimon from trash as bottom digi cards
            placed_count = 0
            for c in list(player.trash_cards):
                if placed_count >= 3:
                    break
                if getattr(c, 'is_digimon', False):
                    traits = getattr(c, 'card_traits', []) or []
                    if any('Ver.2' in t for t in traits):
                        player.trash_cards.remove(c)
                        perm.add_card_source_bottom(c)
                        placed_count += 1
            if placed_count < 3:
                return  # Can't pay cost

            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Ver.2' in t for t in traits)

            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: <Piercing>
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-049 Inherited: Piercing")
        effect2.set_effect_description("Inherited: <Piercing>")
        effect2.is_inherited_effect = True
        effect2._is_piercing = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
