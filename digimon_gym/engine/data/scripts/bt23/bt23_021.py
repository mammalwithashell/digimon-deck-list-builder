from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_021(CardScript):
    """BT23-021"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-021 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Dokamon') or permanent.contains_card_name('Perorimon') or permanent.contains_card_name('Musclemon'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-021 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect1._alt_digi_cost = 3

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] You may link 1 level 3 Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-021 Link 1 level 3 digimon from hand or digivolution cards")
        effect2.set_effect_description("[When Digivolving] [Once Per Turn] You may link 1 level 3 Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect2.set_hash_string("BT23_021_WD/WA")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] You may link 1 level 3 Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-021 Link 1 level 3 digimon from hand or digivolution cards")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] You may link 1 level 3 Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect3.set_hash_string("BT23_021_WD/WA")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenLinked
        # [Your Turn] [Once Per Turn] When this Digimon gets linked, it can't be deleted in battle until your opponent's turn ends.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-021 Gain immunity from battle")
        effect4.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon gets linked, it can't be deleted in battle until your opponent's turn ends.")
        effect4.set_hash_string("BT23_021_WL")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenLinked
        # [When Linking] This Digimon can't be deleted in battle until your opponent's turn ends.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-021 Gain immunity from battle")
        effect5.set_effect_description("[When Linking] This Digimon can't be deleted in battle until your opponent's turn ends.")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
