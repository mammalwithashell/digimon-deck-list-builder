from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_008(CardScript):
    """EX9-008 Biyomon | Lv.3 | Bird/DM/Ver.4
    <Training>
    Inherited: <Raid>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt digi: from Lv.2 w/[DM] trait for cost 0 ---
        effect_alt = ICardEffect()
        effect_alt.set_effect_name("EX9-008 Alt digi from Lv.2 DM")
        effect_alt.set_effect_description("Digivolve: from Lv.2 w/[DM] trait for 0 cost")
        effect_alt._alt_digi_cost = 0
        effect_alt._alt_digi_level = 2
        effect_alt._alt_digi_trait = "DM"

        def condition_alt(context: Dict[str, Any]) -> bool:
            return True
        effect_alt.set_can_use_condition(condition_alt)
        effects.append(effect_alt)

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-008 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Inherited: <Raid>
        effect1 = ICardEffect()
        effect1.set_effect_name("EX9-008 Inherited: Raid")
        effect1.set_effect_description("Inherited: <Raid>")
        effect1.is_inherited_effect = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
