from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_148(CardScript):
    """P-148 Wanyamon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] If this Digimon has the [NSp] trait, <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-148 Draw 1")
        effect0.set_effect_description("[When Attacking] [Once Per Turn] If this Digimon has the [NSp] trait, <Draw 1>.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw1_P_148")
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
