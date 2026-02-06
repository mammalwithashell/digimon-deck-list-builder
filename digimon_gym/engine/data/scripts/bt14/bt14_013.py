from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_013(CardScript):
    """Auto-transpiled from DCGO BT14_013.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] For the turn, when this Digimon would digivolve into a card with [Tyrannomon] in its name, or the [Dinosaur] or [Ceratopsian] trait, reduce the digivolution cost by 1.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-013 Reduce digivolution cost")
        effect0.set_effect_description("[Start of Your Main Phase] For the turn, when this Digimon would digivolve into a card with [Tyrannomon] in its name, or the [Dinosaur] or [Ceratopsian] trait, reduce the digivolution cost by 1.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check trait: "Dinosaur" in target traits
            # Check trait: "Ceratopsian" in target traits
            # Check name: "Tyrannomon" in card name
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Effect"""
            pass  # TODO: implement effect action

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn][Once Per Turn] If this Digimon has [Tyrannomon] in its name, or the [Dinosaur] or [Ceratopsian] trait, it may attack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-013 This Digimon attacks")
        effect1.set_effect_description("[End of Your Turn][Once Per Turn] If this Digimon has [Tyrannomon] in its name, or the [Dinosaur] or [Ceratopsian] trait, it may attack.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Attack_BT14_013")

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check trait: "Dinosaur" in target traits
            # Check trait: "Ceratopsian" in target traits
            # Check name: "Tyrannomon" in card name
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
