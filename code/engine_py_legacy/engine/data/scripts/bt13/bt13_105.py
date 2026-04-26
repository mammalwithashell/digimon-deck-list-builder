from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_105(CardScript):
    """BT13-105 Full Moon Meteor Impact"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Return 1 of your opponent's Digimon to the hand. Then, gain 1 memory for every 4 cards in your opponent's hand.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT13-105 Bounce")
        effect0.set_effect_description("[Main] Return 1 of your opponent's Digimon to the hand. Then, gain 1 memory for every 4 cards in your opponent's hand.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Return 1 of your opponent's Digimon to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT13-105 Bounce")
        effect1.set_effect_description("[Security] Return 1 of your opponent's Digimon to the hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
