from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_024(CardScript):
    """EX7-024 Shoemon | Lv.3, Yellow, Puppet/LIBERATOR, DP 1000, Cost 3

    [Your Turn] When this Digimon would digivolve into a Digimon card with
        the [Puppet] trait, reduce the digivolution cost by 1.
    [Inherited][Your Turn] All of your opponent's Security Digimon get -3000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Your Turn] Digivolve cost -1 into [Puppet] trait ---
        # C#: ChangeDigivolutionCostStaticEffect, NoTiming
        #   permanentCondition: targetPermanent == card.PermanentOfThisCard()
        #   cardCondition: cardSource.IsDigimon && cardSource.ContainsTraits("Puppet")
        #   changeValue: -1, isInheritedEffect: false
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.WhenWouldDigivolve)
        effect0.set_effect_name("EX7-024 Digivolve cost -1 into Puppet trait")
        effect0.set_effect_description(
            "[Your Turn] When this Digimon would digivolve into a Digimon card "
            "with the [Puppet] trait, reduce the digivolution cost by 1."
        )
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the digivolving target card has [Puppet] trait
            target_card = context.get('card_source')
            if target_card:
                traits = getattr(target_card, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)
            return False

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Inherited][Your Turn] Opponent's Security Digimon -3000 DP ---
        # C#: ChangeSecurityDigimonCardDPStaticEffect, NoTiming
        #   cardCondition: cardSource.Owner == card.Owner.Enemy
        #   changeValue: -3000, isInheritedEffect: true
        #   condition: IsExistOnBattleAreaDigimon && IsOwnerTurn
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.NoTiming)
        effect1.set_effect_name("EX7-024 Inherited: Opponent's Security Digimon -3000 DP")
        effect1.set_effect_description(
            "[Inherited][Your Turn] All of your opponent's security Digimon get -3000 DP."
        )
        effect1.is_inherited_effect = True
        effect1.dp_modifier = -3000
        effect1._applies_to_opponent_security_digimon = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            return owner.is_my_turn

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            pass  # Declarative DP modifier for security Digimon — applied by engine

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
