from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT20_038(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Your Turn] When this Digimon would digivolve into a Digimon card with the [ACCEL] trait,
        # reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("Reduce Digivolution Cost")
        effect1.set_effect_description("[Your Turn] When this Digimon would digivolve into a Digimon card with the [ACCEL] trait, reduce the digivolution cost by 1.")
        effect1.set_timing(EffectTiming.BeforePayCost)

        def condition1(context: Dict[str, Any]) -> bool:
            permanent: Optional[Permanent] = effect1.effect_source_permanent
            target_permanent: Optional[Permanent] = context.get("permanent")
            if permanent != target_permanent:
                return False

            target_card: Optional[CardSource] = context.get("card")
            # In a real scenario, check if action is Digivolve
            is_digivolving = context.get("is_digivolving", False)
            if is_digivolving and target_card and "ACCEL" in target_card.card_traits:
                return True
            return False

        def activate1(context: Dict[str, Any]):
            print("BT20-038: Reducing digivolution cost by 1.")
            if "cost_modifier" in context:
                 context["cost_modifier"] -= 1

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # Inherited Effect
        # <Piercing>
        inherited = ICardEffect()
        inherited.set_effect_name("Piercing")
        inherited.set_effect_description("<Piercing> (When this Digimon attacks and deletes an opponent's Digimon in battle, it checks security before the attack ends.)")
        inherited.is_inherited_effect = True
        inherited.is_keyword_effect = True
        inherited.keyword = "Piercing"
        effects.append(inherited)

        return effects
