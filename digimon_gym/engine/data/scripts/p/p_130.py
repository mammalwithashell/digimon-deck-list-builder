from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_130(CardScript):
    """P-130 Lui Ohwada"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may move 1 of your level 3 or higher Digimon from the breeding area to the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("P-130 Move your Digimon to Battle Area")
        effect0.set_effect_description("[On Play] You may move 1 of your level 3 or higher Digimon from the breeding area to the battle area.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnMove
        # [Your Turn] When one of your Digimon moves from the breeding area to the battle area, by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnMove)
        effect1.set_effect_name("P-130 Memory +1")
        effect1.set_effect_description("[Your Turn] When one of your Digimon moves from the breeding area to the battle area, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True

        def condition1_suspend(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must not be suspended already (cost: suspend this Tamer)
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            return True

        # Override condition to check suspend cost
        effect1.set_can_use_condition(condition1_suspend)

        def process1(ctx: Dict[str, Any]):
            """By suspending this Tamer, gain 1 memory."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not (player and perm):
                return
            # Cost: suspend this Tamer
            perm.suspend()
            # Reward: gain 1 memory
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("P-130 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
