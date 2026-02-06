from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_087(CardScript):
    """Auto-transpiled from DCGO BT14_087.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-087 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True
        # Security effect: play this card without paying cost
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-087 Memory +1")
        effect1.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Gain 1 memory"""
            # card.owner.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Mind Link> with 1 of your Digimon with the [Dark Animal] or [SoC] trait. (Place this Tamer as that Digimon's bottom digivolution card if there are no Tamer cards in its digivolution cards.)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-087 Mind Link")
        effect2.set_effect_description("[Main] <Mind Link> with 1 of your Digimon with the [Dark Animal] or [SoC] trait. (Place this Tamer as that Digimon's bottom digivolution card if there are no Tamer cards in its digivolution cards.)")

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check trait: "DarkAnimal" in target traits
            # Check trait: "Dark Animal" in target traits
            # Check trait: "SoC" in target traits
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)

        def process2():
            """Action: Mind Link"""
            # mind_link(tamer, digimon)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-087 Blocker")
        effect3.set_effect_description("Blocker")
        # TODO: Blocker keyword - this Digimon can block
        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: alliance
        # Alliance
        effect4 = ICardEffect()
        effect4.set_effect_name("BT14-087 Alliance")
        effect4.set_effect_description("Alliance")
        # TODO: Alliance keyword
        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # [End of All Turns] You may play 1 [Eiji Nagasumi] from this Digimon's digivolution cards without paying the cost.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT14-087 Play 1 [Eiji Nagasumi] from this Digimon's digivolution cards")
        effect5.set_effect_description("[End of All Turns] You may play 1 [Eiji Nagasumi] from this Digimon's digivolution cards without paying the cost.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True

        def condition5(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect5.set_can_use_condition(condition5)

        def process5():
            """Action: Play Card"""
            # play_card_from_hand_or_trash()

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
