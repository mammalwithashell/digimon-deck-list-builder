from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_001(CardScript):
    """BT23-001 Flickmon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] If this Digimon has the [Appmon] trait, <Draw 1>
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-001 Draw 1")
        effect0.set_effect_description("[When Attacking] [Once Per Turn] If this Digimon has the [Appmon] trait, <Draw 1>")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT23_001_Draw")
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
