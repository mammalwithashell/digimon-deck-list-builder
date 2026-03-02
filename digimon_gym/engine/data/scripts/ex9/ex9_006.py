from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_006(CardScript):
    """EX9-006 Pagumon | Lv.2 Purple Digi-Egg | Lesser/DM/Ver.5
    NOTE: EX9 set not yet in CardDatabase. Script ready for when data is added.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [When Attacking][Once Per Turn]
        #     Trash bottom digivolution card, digivolve into Ver.5 from trash ---
        # NOTE: Digivolving from trash with cost reduction via trashing own
        # digivolution card is a complex mechanic. Marked PARTIAL.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnTappedAnyone)
        effect0.set_effect_name("EX9-006 Inherited: Trash bottom evo card, digivolve from trash")
        effect0.set_effect_description(
            "Inherited: [When Attacking][Once Per Turn] By trashing this "
            "Digimon's bottom face-down digivolution card, this Digimon may "
            "digivolve into a [Ver.5] trait Digimon card in the trash with "
            "the digivolution cost reduced by 1."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("evo_from_trash_EX9_006")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash bottom digi card, digivolve into Ver.5 from trash"""
            # descriptive-tagged: digivolving from trash into a trait-specific
            # Digimon with cost reduction is not supported by the engine
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
