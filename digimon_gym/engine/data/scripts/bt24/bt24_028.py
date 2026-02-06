from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_028(CardScript):
    """Auto-transpiled from DCGO BT24_028.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains <Blocker>
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-028 By placing 1 level 5 or lower [TS digimon] as bottom source card, gain battle immunity & <Blocker>")
        effect0.set_effect_description("[On Play] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains <Blocker>")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains <Blocker>
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-028 By placing 1 level 5 or lower [TS digimon] as bottom source card, gain battle immunity & <Blocker>")
        effect1.set_effect_description("[When Digivolving] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains <Blocker>")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUnTappedAnyone
        # [Your Turn] When this Digimon unsuspends, it may digivolve into [Neptunemon] in the hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-028 You may digivolve into [Neptunemon]")
        effect2.set_effect_description("[Your Turn] When this Digimon unsuspends, it may digivolve into [Neptunemon] in the hand without paying the cost.")

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass  # TODO: digivolve effect needs card selection

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-028 Play 1 level 4 or lower blue [TS] digimon in digivolution sources")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_028_ESS")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
