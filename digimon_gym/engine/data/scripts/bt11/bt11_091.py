from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_091(CardScript):
    """BT11-091 Taiga"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-091 All your Digimon DP modifier")
        effect0.set_effect_description("All your Digimon DP modifier")
        effect0.dp_modifier = 1000
        effect0._applies_to_all_own_digimon = True

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # [Your Turn] When one of your green Digimon would digivolve into a level 5 or higher card, by suspending this Tamer, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-091 Digivolution Cost -1")
        effect1.set_effect_description("[Your Turn] When one of your green Digimon would digivolve into a level 5 or higher card, by suspending this Tamer, reduce the digivolution cost by 1.")
        effect1.is_optional = True
        effect1.set_hash_string("Digisorption-1_BT11_091")
        effect1.cost_reduction = 1

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -1, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level < 5:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-091 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
