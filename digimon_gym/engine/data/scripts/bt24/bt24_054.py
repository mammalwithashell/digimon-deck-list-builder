from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_054(CardScript):
    """BT24-054 Ryudamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-054 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Kyokyomon] with [DigiPolice] trait for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Kyokyomon"
        effect0._alt_digi_trait = "DigiPolice"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Kyokyomon'))):
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('DigiPolice' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('SEEKERS' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your [Shuu Yulin]s are played, this Digimon may digivolve into [Hisyaryumon] in the hand for a digivolution cost of 3, ignoring digivolution requirements.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-054 Digivolve into a [Hisyaryumon]] in the hand")
        effect1.set_effect_description("[Your Turn] When any of your [Shuu Yulin]s are played, this Digimon may digivolve into [Hisyaryumon] in the hand for a digivolution cost of 3, ignoring digivolution requirements.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Shuu Yulin'))):
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
                if not (any('Hisyaryumon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon suspendeds, suspend 1 of your opponent's Digimon or Tamers with as high or lower a play cost as this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-054 Suspend opponent's Digimon or Tamers with play cost less than this Digimon.")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspendeds, suspend 1 of your opponent's Digimon or Tamers with as high or lower a play cost as this Digimon.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_054_Inherited")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
