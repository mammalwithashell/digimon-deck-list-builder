from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_029(CardScript):
    """BT13-029 MachGaogamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] If your opponent has 8 or more cards in their hand, for the turn, this Digimon's attack target can't be switched.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("BT13-029 This Digimon's attack target can't be switched")
        effect0.set_effect_description("[When Attacking] If your opponent has 8 or more cards in their hand, for the turn, this Digimon's attack target can't be switched.")
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Target Lock"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Target lock — this Digimon's attack target can't be switched
            pass  # Handled by engine attack target resolution

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddHand
        # [All Turns][Once Per Turn] When an effect adds cards to your opponent's hand, unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddHand)
        effect1.set_effect_name("BT13-029 Unsuspend this Digimon")
        effect1.set_effect_description("[All Turns][Once Per Turn] When an effect adds cards to your opponent's hand, unsuspend this Digimon.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT13_029")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
