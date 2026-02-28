from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_037(CardScript):
    """EX8-037 Sakuyamon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-037 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 1
        effect0._alt_digi_cost = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If [Sakuyamon] or [X Antibody] is in this Digimon's digivolution cards, play 1 [Uka-no-Mitama] (Digimon/Yellow/9000 DP/<Rush>) Token.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-037 Play a Token")
        effect1.set_effect_description("[When Digivolving] If [Sakuyamon] or [X Antibody] is in this Digimon's digivolution cards, play 1 [Uka-no-Mitama] (Digimon/Yellow/9000 DP/<Rush>) Token.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [Your Turn] [Once Per Turn] When any of your Digimon attack, you may use 1 single-color Option card with a cost of 5 or less from your hand without paying the cost. If you did, 1 of your Digimon unsuspends.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX8-037 Play 1 single-color option card")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When any of your Digimon attack, you may use 1 single-color Option card with a cost of 5 or less from your hand without paying the cost. If you did, 1 of your Digimon unsuspends.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("PlayOption_EX8_037")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
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
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
