from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_052(CardScript):
    """BT24-052 Keramon (X Antibody) | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-052 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Keramon] for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Keramon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Keramon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnMove
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-052 Effect")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-052 Effect")
        effect2.set_effect_description("Effect")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon with [Diaboromon] in its text would leave the battle area, by deleting 1 of your other [Diaboromon], it doesn't leave.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-052 Delete 1 of your other [Diaboromon] to prevent this Digimon from leaving")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When this Digimon with [Diaboromon] in its text would leave the battle area, by deleting 1 of your other [Diaboromon], it doesn't leave.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Substitute_BT24_052")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Diaboromon' in text):
                    return False
            else:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Diaboromon'))):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
