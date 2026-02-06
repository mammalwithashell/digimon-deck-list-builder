from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_074(CardScript):
    """Auto-transpiled from DCGO BT14_074.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By trashing 1 card in your hand, <Draw 1>. If [Eiji Nagasumi] is in this Digimon's digivolution cards, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-074 Trash 1 card from hand to Draw 1, and gain Memory +1")
        effect0.set_effect_description("[When Attacking] By trashing 1 card in your hand, <Draw 1>. If [Eiji Nagasumi] is in this Digimon's digivolution cards, gain 1 memory.")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When you play a card with the [Dark Animal] or [SoC] trait, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-074 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When you play a card with the [Dark Animal] or [SoC] trait, gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT14_074")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
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
