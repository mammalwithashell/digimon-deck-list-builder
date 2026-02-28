from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_112(CardScript):
    """BT10-112 Jesmon GX | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-112 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 5
        effect0._alt_digi_cost = 5

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 card with [Royal Knight] in its traits and a play cost of 13 or less from your hand or trash under this Digimon as its bottom digivolution card. Activate 1 of that Digimon's [When Digivolving] effects as an effect of this Digimon. Then, <Blitz>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-112 Place 1 card to digivolution cards to activate effects and Blitz")
        effect1.set_effect_description("[When Digivolving] You may place 1 card with [Royal Knight] in its traits and a play cost of 13 or less from your hand or trash under this Digimon as its bottom digivolution card. Activate 1 of that Digimon's [When Digivolving] effects as an effect of this Digimon. Then, <Blitz>.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-112 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect3 = ICardEffect()
        effect3.set_effect_name("BT10-112 Security Attack +1")
        effect3.set_effect_description("Security Attack +1")
        effect3._security_attack_modifier = 1

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
