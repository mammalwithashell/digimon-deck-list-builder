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
            """Action: Trash top security, then gain 2 memory.

            Use player.trash_security_card() (not raw list pop) so that
            OnDiscardSecurity / OnLoseSecurity observers fire correctly.
            Resolution order matches card text: trash first, then memory.
            """
            player = ctx.get('player')
            if not player:
                return
            # Trash the top card of the security stack (index 0 == top)
            if player.security_cards:
                top_sec = player.security_cards[0]
                player.trash_security_card(top_sec)
            # Then, gain 2 memory
            player.add_memory(2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
