from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_024(CardScript):
    """Auto-transpiled from DCGO BT24_024.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: armor_purge
        # Armor Purge
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-024 Armor Purge")
        effect0.set_effect_description("Armor Purge")
        effect0._is_armor_purge = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # Cost -2, Play Card, Trash From Hand
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-024 Play one [Ts] Tamer for cost reduced by 2.")
        effect1.set_effect_description("Cost -2, Play Card, Trash From Hand")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_024_WA_Play_Tamer")
        effect1.is_on_attack = True
        effect1.cost_reduction = 2

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -2, Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])
            # Cost reduction handled via cost_reduction property

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
