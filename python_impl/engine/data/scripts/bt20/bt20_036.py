from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT20_036(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Rule: When this card would be played, if you have a Digimon with the [ACCEL] trait, reduce the play cost by 5.
        cost_reduction = ICardEffect()
        cost_reduction.set_effect_name("Cost Reduction Rule")
        cost_reduction.set_effect_description("When this card would be played, if you have a Digimon with the [ACCEL] trait, reduce the play cost by 5.")
        cost_reduction.set_timing(EffectTiming.BeforePayCost)

        def cost_reduction_condition(context: Dict[str, Any]) -> bool:
            # Check for ACCEL trait on field
            # player = card.owner
            # for permanent in player.battle_area:
            #     if "ACCEL" in permanent.card_source.card_traits:
            #         return True
            return False # Placeholder

        def cost_reduction_activate(context: Dict[str, Any]):
            print("BT20-036 Rule: Reducing play cost by 5.")

        cost_reduction.set_can_use_condition(cost_reduction_condition)
        cost_reduction.set_on_process_callback(cost_reduction_activate)
        effects.append(cost_reduction)

        # [On Play] [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon.
        # Then, 1 of their Digimon gets -5000 DP until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("De-Digivolve and DP Minus")
        effect1.set_effect_description("[On Play] [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, 1 of their Digimon gets -5000 DP until the end of their turn.")
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        def activate1(context: Dict[str, Any]):
            print("BT20-036 Effect 1: De-Digivolve 2 and -5000 DP.")
            # target.de_digivolve(2)
            # target2.dp_modifier -= 5000

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # [End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve
        # into a Digimon card with [Chaosmon] in its name in the hand.
        # Then, the DNA digivolved Digimon may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("DNA Digivolve into Chaosmon")
        effect2.set_effect_description("[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card with [Chaosmon] in its name in the hand. Then, the DNA digivolved Digimon may attack.")
        effect2.set_timing(EffectTiming.OnEndTurn)

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        def activate2(context: Dict[str, Any]):
            print("BT20-036 Effect 2: DNA Digivolve into Chaosmon.")

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(activate2)
        effects.append(effect2)

        # Inherited Effect
        # [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks,
        # you may change the attack target to this Digimon.
        inherited = ICardEffect()
        inherited.set_effect_name("Redirect Attack")
        inherited.set_effect_description("[Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, you may change the attack target to this Digimon.")
        inherited.is_inherited_effect = True
        inherited.set_timing(EffectTiming.OnDeclaration)
        inherited.set_limit_once_per_turn()

        def inherited_condition(context: Dict[str, Any]) -> bool:
            attacker: Optional[Permanent] = context.get("attacker")
            if attacker and attacker.player != inherited.effect_source_permanent.player:
                return True
            return False

        def inherited_activate(context: Dict[str, Any]):
            print("BT20-036 Inherited: Redirecting attack.")

        inherited.set_can_use_condition(inherited_condition)
        inherited.set_on_process_callback(inherited_activate)
        effects.append(inherited)

        return effects
