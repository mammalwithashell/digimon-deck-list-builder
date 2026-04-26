from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_069(CardScript):
    """BT14-069 Gazimon | Lv.3
    Inherited Effect [On Deletion] Gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited [On Deletion] Gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT14-069 Inherited: On Deletion gain 1 memory")
        effect0.set_effect_description(
            "Inherited: [On Deletion] Gain 1 memory."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_deletion = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
