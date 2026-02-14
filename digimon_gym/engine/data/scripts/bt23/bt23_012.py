from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_012(CardScript):
    """BT23-012"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-012 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your Digimon gains <Raid> for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-012 Give 1 digimon Raid")
        effect1.set_effect_description("[On Play] 1 of your Digimon gains <Raid> for the turn.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your Digimon gains <Raid> for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-012 Give 1 digimon Raid")
        effect2.set_effect_description("[When Digivolving] 1 of your Digimon gains <Raid> for the turn.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 4 or lower Digimon card with the [CS] trait or [Avian], [Bird], [Beast], [Animal] or [Sovereign] in any of its traits (other than [Sea Animal]) from your hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-012 Play 1 level 4 or lower digimon from hand")
        effect3.set_effect_description("[On Deletion] You may play 1 level 4 or lower Digimon card with the [CS] trait or [Avian], [Bird], [Beast], [Animal] or [Sovereign] in any of its traits (other than [Sea Animal]) from your hand without paying the cost.")
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 4 or lower Digimon card with the [CS] trait or [Avian], [Bird], [Beast], [Animal] or [Sovereign] in any of its traits (other than [Sea Animal]) from your hand without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-012 Play 1 level 4 or lower digimon from hand")
        effect4.set_effect_description("[On Deletion] You may play 1 level 4 or lower Digimon card with the [CS] trait or [Avian], [Bird], [Beast], [Animal] or [Sovereign] in any of its traits (other than [Sea Animal]) from your hand without paying the cost.")
        effect4.is_inherited_effect = True
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
