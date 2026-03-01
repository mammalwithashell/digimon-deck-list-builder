from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_063(CardScript):
    """BT22-063 Alphamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-063 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-063 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Kyoko Kuremi] for cost 5
        effect1._alt_digi_cost = 5
        effect1._alt_digi_name = "Kyoko Kuremi"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Kyoko Kuremi'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-063 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: reboot
        # Reboot
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-063 Reboot")
        effect3.set_effect_description("Reboot")
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT22-063 -5K DP to 1 digimon")
        effect4.set_effect_description("[On Play] 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT22-063 -5K DP to 1 digimon")
        effect5.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnAllyAttack)
        effect6.set_effect_name("BT22-063 -5K DP to 1 digimon")
        effect6.set_effect_description("[When Attacking] 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect6.is_on_attack = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon suspends, if [Kyoko Kuremi] is in this Digimon's digivolution cards or if this Digimon's stack has 2 or more same-level cards, it gets +3000 DP until your opponent's turn ends. Then, this Digimon unsuspends.
        effect7 = ICardEffect()
        effect7.set_timing(EffectTiming.OnTappedAnyone)
        effect7.set_effect_name("BT22-063 Gain 3K DP & Unsuspend")
        effect7.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspends, if [Kyoko Kuremi] is in this Digimon's digivolution cards or if this Digimon's stack has 2 or more same-level cards, it gets +3000 DP until your opponent's turn ends. Then, this Digimon unsuspends.")
        effect7.set_max_count_per_turn(1)
        effect7.set_hash_string("BT22_063_UnSuspend")

        effect = effect7  # alias for condition closure
        def condition7(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect7.set_can_use_condition(condition7)

        def process7(ctx: Dict[str, Any]):
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

        effect7.set_on_process_callback(process7)
        effects.append(effect7)

        return effects
