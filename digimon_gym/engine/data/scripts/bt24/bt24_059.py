from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_059(CardScript):
    """Auto-transpiled from DCGO BT24_059.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-059 Effect")
        effect0.set_effect_description("Effect")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-059 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Reveal the top 3 cards of your deck. You may play 1 cost 7 or lower [TS] trait card suspended among them without paying the cost. Trash the rest.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-059 Reveal 3, maybe play 7 cost [TS] trait, trash rest.")
        effect2.set_effect_description("[On Deletion] Reveal the top 3 cards of your deck. You may play 1 cost 7 or lower [TS] trait card suspended among them without paying the cost. Trash the rest.")
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this Digimon's bottom digivolution card, it unsuspends.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-059 Place 1 of your other Digimon as this Digimon's bottom digivolution card to unsuspend this Digimon.")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this Digimon's bottom digivolution card, it unsuspends.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_059_Inherited")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
