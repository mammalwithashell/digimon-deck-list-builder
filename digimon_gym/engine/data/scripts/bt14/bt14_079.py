from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_079(CardScript):
    """Auto-transpiled from DCGO BT14_079.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 level 3 or lower card with the [Dark Animal] or [SoC] trait from your trash without paying the cost. If [Eiji Nagasumi] is in this Digimon's digivolution cards, add 1 to the level this effect may choose.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-079 Play 1 Digimon from trash")
        effect0.set_effect_description("[When Digivolving] You may play 1 level 3 or lower card with the [Dark Animal] or [SoC] trait from your trash without paying the cost. If [Eiji Nagasumi] is in this Digimon's digivolution cards, add 1 to the level this effect may choose.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check trait: "Dark Animal" in target traits
            # Check trait: "DarkAnimal" in target traits
            # Check trait: "SoC" in target traits
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Play Card"""
            # play_card_from_hand_or_trash()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By trashing 1 card in your hand, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-079 Trash 1 card from hand to gain Memory +1")
        effect1.set_effect_description("[When Attacking] By trashing 1 card in your hand, gain 1 memory.")
        effect1.is_optional = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Gain 1 memory, Trash From Hand"""
            # card.owner.add_memory(1)
            # card.owner.trash_from_hand(count)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When you play a card with the [Dark Animal] or [SoC] trait, you may unsuspend this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-079 Unsuspend this Digimon")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When you play a card with the [Dark Animal] or [SoC] trait, you may unsuspend this Digimon.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Unsupend_BT14_079")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check trait: "Dark Animal" in target traits
            # Check trait: "DarkAnimal" in target traits
            # Check trait: "SoC" in target traits
            return True  # TODO: implement condition checks against game state

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
