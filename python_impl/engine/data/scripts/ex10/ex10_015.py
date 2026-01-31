from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_015(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # On Deletion: Save
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-015: <Save>")
        effect1.timing = EffectTiming.OnDestroyedAnyone
        effects.append(effect1)

        # Start of Main Phase
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-015: Trash/Draw/Suspend")
        effect2.timing = EffectTiming.OnStartMainPhase

        def activate2():
            # Trash 1 with Save from Hand
            # Draw 1
            # Suspend 1 Opponent Digimon
            print("Trashed Save card, Drew 1, Suspended Opponent Digimon.")

        effect2.set_on_process_callback(activate2)
        effects.append(effect2)

        # ESS: Piercing
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-015 ESS: <Piercing>")
        effect3.is_inherited_effect = True
        effect3.timing = EffectTiming.OnDetermineDoSecurityCheck
        effects.append(effect3)

        return effects
