from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_012(CardScript):
    """BT24-012 Dimetromon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-012 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your other Digimon with the [Reptile] or [Dragonkin] trait would leave the battle area by your opponent's effects, by returning this Digimon to the hand, they don't leave.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-012 By bouncing to hand, others don't leave")
        effect1.set_effect_description("[All Turns] When any of your other Digimon with the [Reptile] or [Dragonkin] trait would leave the battle area by your opponent's effects, by returning this Digimon to the hand, they don't leave.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnLoseSecurity
        # Gain 1 memory
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-012 Gain 1 memory")
        effect2.set_effect_description("Gain 1 memory")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("GainMemory_BT24-012")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
