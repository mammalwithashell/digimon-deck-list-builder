from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_001(CardScript):
    """EX10-001 Flickmon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnLinkCardDiscarded
        # [Your Turn] [Once Per Turn] When effects trash any of this Digimon's link cards, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-001 Gain +1 Memory")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When effects trash any of this Digimon's link cards, gain 1 memory.")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("GainMemory_EX10-001")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
