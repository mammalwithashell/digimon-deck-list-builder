from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_095(CardScript):
    """BT23-095"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-095 Ignore color requirements")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Return 1 of your opponent's suspended Digimon to the bottom of the deck. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-095 Bottom-deck 1 suspended digimon, then place in battle area")
        effect1.set_effect_description("[Main] Return 1 of your opponent's suspended Digimon to the bottom of the deck. Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [Your Turn] When one of your [CS] trait Digimon attacks, <Delay> \r\n・Return 1 of your opponent's suspended Digimon to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-095 Bottom-deck 1 suspended digimon")
        effect2.set_effect_description("[Your Turn] When one of your [CS] trait Digimon attacks, <Delay> \r\n・Return 1 of your opponent's suspended Digimon to the bottom of the deck.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Return 1 of your opponent's suspended Digimon to the bottom of the deck. Then, place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-095 Bottom-deck 1 suspended digimon, then place in battle area")
        effect3.set_effect_description("[Security] Return 1 of your opponent's suspended Digimon to the bottom of the deck. Then, place this card in the battle area.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
