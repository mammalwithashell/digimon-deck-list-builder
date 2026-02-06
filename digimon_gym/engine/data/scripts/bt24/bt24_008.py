from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_008(CardScript):
    """Auto-transpiled from DCGO BT24_008.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing 1 card with the [Reptile], [Dragonkin] or [LIBERATOR] trait in your hand, <Draw 2>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-008 Trash 1 [Reptile], [Dragonkin] or [LIBERATOR] trait to <Draw 2>")
        effect0.set_effect_description("[On Play] By trashing 1 card with the [Reptile], [Dragonkin] or [LIBERATOR] trait in your hand, <Draw 2>.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 2, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(2)
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # Gain 1 memory
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-008 Gain 1 memory")
        effect1.set_effect_description("Gain 1 memory")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("GainMemory_BT24_008")

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
