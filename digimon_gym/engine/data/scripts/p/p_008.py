from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_008(CardScript):
    """P-008 WereGarurumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] If this Digimon has a [Garurumon] digivolution card�Cunsuspend this Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-008 Unsuspend this Digimon")
        effect0.set_effect_description("[When Attacking][Once Per Turn] If this Digimon has a [Garurumon] digivolution card�Cunsuspend this Digimon.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Unsuspend_P_008")
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
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

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect1 = ICardEffect()
        effect1.set_effect_name("P-008 Security Attack +1")
        effect1.set_effect_description("Security Attack +1")
        effect1.is_inherited_effect = True
        effect1._security_attack_modifier = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
