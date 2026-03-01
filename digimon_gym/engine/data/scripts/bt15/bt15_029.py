from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_029(CardScript):
    """BT15-029 MegaSeadramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 of your other blue Digimon as this Digimon's bottom digivolution card, return 1 of your opponent's Digimon whose level is less than or equal to the placed card's level to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT15-029 Place 1 Digimon card to digivolution cards to return 1 of your opponent's Digimon whose level is less than or equal to the placed card's level to the bottom of the deck.")
        effect0.set_effect_description("[On Play] By placing 1 of your other blue Digimon as this Digimon's bottom digivolution card, return 1 of your opponent's Digimon whose level is less than or equal to the placed card's level to the bottom of the deck.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 of your other blue Digimon as this Digimon's bottom digivolution card, return 1 of your opponent's Digimon whose level is less than or equal to the placed card's level to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT15-029 Place 1 Digimon card to digivolution cards to return 1 of your opponent's Digimon whose level is less than or equal to the placed card's level to the bottom of the deck.")
        effect1.set_effect_description("[When Digivolving] By placing 1 of your other blue Digimon as this Digimon's bottom digivolution card, return 1 of your opponent's Digimon whose level is less than or equal to the placed card's level to the bottom of the deck.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] By placing 1 of your other blue Digimon as this Digimon's bottom digivolution card, unsuspend this Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("BT15-029 Unsuspend this Digimon, by placing 1 blue Digimon as bottom source")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] By placing 1 of your other blue Digimon as this Digimon's bottom digivolution card, unsuspend this Digimon.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Unsuspend_BT15_020")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
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
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
