from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT4_001(CardScript):
    """BT4-001 Sakuttomon | Lv.2 Red Digi-Egg

    Inherited Effect:
    [When Attacking] [Once Per Turn] If this Digimon is level 7,
        gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [When Attacking][Once Per Turn] Lv.7 memory +1 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAllyAttack)
        effect0.set_effect_name("BT4-001 Inherited: Lv.7 memory +1 on attack")
        effect0.set_effect_description(
            "[When Attacking] [Once Per Turn] If this Digimon is level 7, "
            "gain 1 memory."
        )
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("MemoryGain_BT4_001")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm or not perm.is_digimon:
                return False
            # Must be level 7
            level = getattr(perm, 'level', None)
            if level != 7:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
