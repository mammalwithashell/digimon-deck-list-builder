from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_090(CardScript):
    """BT15-090 Fox Fire"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Return 1 of your opponent's level 4 or lower Digimon to the hand. If you have a Digimon with [Gabumon] or [Garurumon] in its name, return 1 of your opponent's Digimon with the lowest level to the hand instead.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-090 Return 1 of your opponent's level 4 or lower Digimon to the hand")
        effect0.set_effect_description("[Main] Return 1 of your opponent's level 4 or lower Digimon to the hand. If you have a Digimon with [Gabumon] or [Garurumon] in its name, return 1 of your opponent's Digimon with the lowest level to the hand instead.")

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
                if p.level is None or p.level > 4:
                    return False
                if not (p.contains_card_name('Gabumon')):
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-090 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
