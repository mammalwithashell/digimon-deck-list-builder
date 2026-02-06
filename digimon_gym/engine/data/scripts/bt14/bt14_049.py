from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_049(CardScript):
    """Auto-transpiled from DCGO BT14_049.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-049 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 1 of your opponent's Digimon. Then, you may return 1 of your opponent's suspended Digimon with 5000 DP or less to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-049 Suspend 1 Digimon and return 1 Digimon to the bottom of Deck")
        effect1.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon. Then, you may return 1 of your opponent's suspended Digimon with 5000 DP or less to the bottom of the deck.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon. Then, you may return 1 of your opponent's suspended Digimon with 5000 DP or less to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-049 Suspend 1 Digimon and return 1 Digimon to the bottom of Deck")
        effect2.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon. Then, you may return 1 of your opponent's suspended Digimon with 5000 DP or less to the bottom of the deck.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
