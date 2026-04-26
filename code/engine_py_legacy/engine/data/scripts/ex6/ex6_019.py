from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_019(CardScript):
    """EX6-019 Angemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: barrier
        # Barrier
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-019 Barrier")
        effect0.set_effect_description("Barrier")
        effect0._is_barrier = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] If this Digimon has the [Angel]/[Archangel]/[Three Great Angels] trait, [Draw 1].
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("EX6-019 Draw 1")
        effect1.set_effect_description("[When Attacking] [Once Per Turn] If this Digimon has the [Angel]/[Archangel]/[Three Great Angels] trait, [Draw 1].")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Draw1_EX6_019")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
