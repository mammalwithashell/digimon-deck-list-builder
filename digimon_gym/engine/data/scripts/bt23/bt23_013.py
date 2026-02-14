from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_013(CardScript):
    """BT23-013"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-013 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [SaviorHuckmon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "SaviorHuckmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('SaviorHuckmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-013 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Huckmon] for cost 5
        effect1._alt_digi_cost = 5
        effect1._alt_digi_name = "Huckmon"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Huckmon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: alliance
        # Alliance
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-013 Alliance")
        effect2.set_effect_description("Alliance")
        effect2._is_alliance = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 [Atho, Ren� & Por] Token or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-013 Play 1 [Atho, Ren� & Por] token or 1 [Sistermon] in name digimon from hand or trash")
        effect3.set_effect_description("[When Digivolving] You may play 1 [Atho, Ren� & Por] Token or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] You may play 1 [Atho, Ren� & Por] Token or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-013 Play 1 [Atho, Ren� & Por] token or 1 [Sistermon] in name digimon from hand or trash")
        effect4.set_effect_description("[When Attacking] You may play 1 [Atho, Ren� & Por] Token or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.")
        effect4.is_on_attack = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When any of your other Digimon are played, this Digimon may attack.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-013 This digimon may attack")
        effect5.set_effect_description("[Your Turn] [Once Per Turn] When any of your other Digimon are played, this Digimon may attack.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT23_013_YT")
        effect5.is_on_play = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
