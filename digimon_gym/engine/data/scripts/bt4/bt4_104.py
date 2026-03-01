from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT4_104(CardScript):
    """BT4-104 Blinding Ray"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Trash the top card of your security stack. Then, gain 2 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT4-104 Trash security, gain 2 memory")
        effect0.set_effect_description("[Main] Trash the top card of your security stack. Then, gain 2 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash top security, gain 2 memory"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            if player.security_cards:
                trashed = player.security_cards.pop()
                player.trash_cards.append(trashed)
            player.add_memory(2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
