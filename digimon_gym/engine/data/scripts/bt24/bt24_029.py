from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_029(CardScript):
    """Auto-transpiled from DCGO BT24_029.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 play cost 5 or lower card with the [Sea Beast] or [TS] trait or [Aqua] or [Sea Animal] in any of its traits from your hand as this Digimon's bottom digivolution card, 1 of your opponent's Digimon or Tamers can't suspend until their turn ends.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-029 By placing 1 5 or lower play cost [TS]/[Sea Beast]/[Aqua]/[Sea Animal] card as bottom source, 1 digimon/tamer cant suspend")
        effect0.set_effect_description("[On Play] By placing 1 play cost 5 or lower card with the [Sea Beast] or [TS] trait or [Aqua] or [Sea Animal] in any of its traits from your hand as this Digimon's bottom digivolution card, 1 of your opponent's Digimon or Tamers can't suspend until their turn ends.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 play cost 5 or lower card with the [Sea Beast] or [TS] trait or [Aqua] or [Sea Animal] in any of its traits from your hand as this Digimon's bottom digivolution card, 1 of your opponent's Digimon or Tamers can't suspend until their turn ends.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-029 By placing 1 5 or lower play cost [TS]/[Sea Beast]/[Aqua]/[Sea Animal] card as bottom source, 1 digimon/tamer cant suspend")
        effect1.set_effect_description("[When Digivolving] By placing 1 play cost 5 or lower card with the [Sea Beast] or [TS] trait or [Aqua] or [Sea Animal] in any of its traits from your hand as this Digimon's bottom digivolution card, 1 of your opponent's Digimon or Tamers can't suspend until their turn ends.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] [Once Per Turn] You may play 1 play cost 5 or lower [TS] trait card from this Digimon's digivolution cards without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-029 Play 1 5 cost or lower [TS] card from digivolution sources")
        effect2.set_effect_description("[End of Attack] [Once Per Turn] You may play 1 play cost 5 or lower [TS] trait card from this Digimon's digivolution cards without paying the cost.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_029_EOA")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-029 Play 1 level 4 or lower blue [TS] digimon in digivolution sources")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_029_ESS")
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
