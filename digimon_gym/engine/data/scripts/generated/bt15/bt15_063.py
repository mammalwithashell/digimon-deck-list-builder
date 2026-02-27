from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_063(CardScript):
    """BT15-063 Hisyaryumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-063 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-063 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect1._alt_digi_cost = 3

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns][Once Per Turn] When an effect suspends another Digimon or Tamer, unsuspend 1 of your Digimon with the [Beast Dragon] or [DigiPolice] trait.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-063 Unsuspend a Digimon with the [Beast Dragon] or [DigiPolice] trait.")
        effect2.set_effect_description("[All Turns][Once Per Turn] When an effect suspends another Digimon or Tamer, unsuspend 1 of your Digimon with the [Beast Dragon] or [DigiPolice] trait.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("unsuspend_BT15_063")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] When an effect suspends another Digimon or Tamer, if this Digimon has a Tamer card with the [DigiPolice] trait in it's digivolution cards, you may digivole this Digimon into a Digimon card with the [Beast Dragon] or [DigiPolice] trait from your hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT15-063 This Digimon digivolves")
        effect3.set_effect_description("[All Turns] When an effect suspends another Digimon or Tamer, if this Digimon has a Tamer card with the [DigiPolice] trait in it's digivolution cards, you may digivole this Digimon into a Digimon card with the [Beast Dragon] or [DigiPolice] trait from your hand without paying the cost.")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('DigiPolice' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
