from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_061(CardScript):
    """BT23-061"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your Digimon with the [Ghost] trait gains <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-061 1 [Ghost] digimon gain <Blocker>")
        effect0.set_effect_description("[On Play] 1 of your Digimon with the [Ghost] trait gains <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] 1 of your Digimon with the [Ghost] trait gains <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-061 1 [Ghost] digimon gain <Blocker>")
        effect1.set_effect_description("[On Deletion] 1 of your Digimon with the [Ghost] trait gains <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-061 Memory +1")
        effect2.set_effect_description("[On Deletion] Gain 1 memory.")
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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
