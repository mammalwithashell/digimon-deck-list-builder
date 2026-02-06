from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_022(CardScript):
    """Auto-transpiled from DCGO BT24_022.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: jamming
        # Jamming
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-022 Jamming")
        effect0.set_effect_description("Jamming")
        effect0._is_jamming = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-022 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-022 Effect")
        effect2.set_effect_description("Effect")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnUnTappedAnyone
        # Draw 1
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-022 If you have 7 or fewer cards in hand, <Draw 1>.")
        effect3.set_effect_description("Draw 1")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_022_YT_Draw1")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
