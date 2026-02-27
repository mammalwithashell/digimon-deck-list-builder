from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_017(CardScript):
    """BT11-017 Marsmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: raid
        # Raid
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-017 Raid")
        effect0.set_effect_description("Raid")
        effect0.is_on_attack = True
        effect0._is_raid = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blitz
        # Blitz
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-017 Blitz")
        effect1.set_effect_description("Blitz")
        effect1.is_on_play = True
        effect1._is_blitz = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [Your Turn][Once Per Turn] When one of your Digimon's attack targets is switched, unsuspend this Digimon, and gain 1 memory for each red Tamer you have in play.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-017 Unsuspend this Digimon and gain Memory")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When one of your Digimon's attack targets is switched, unsuspend this Digimon, and gain 1 memory for each red Tamer you have in play.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Unsuspend_BT11_017")

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
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
