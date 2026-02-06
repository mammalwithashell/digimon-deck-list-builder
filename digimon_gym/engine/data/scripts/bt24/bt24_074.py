from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_074(CardScript):
    """Auto-transpiled from DCGO BT24_074.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-074 Effect")
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
        effect1.set_effect_name("BT24-074 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 4 or lower Digimon card with [Seadramon] in it's name or the [TS] trait from your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-074 You may play 1 level 4 or lower Digimon")
        effect2.set_effect_description("[On Deletion] You may play 1 level 4 or lower Digimon card with [Seadramon] in it's name or the [TS] trait from your trash without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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
        # [When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this Digimon's bottom digivolution card, it unsuspends.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-074 Place 1 of your other Digimon as this Digimon's bottom digivolution card to unsuspend this Digimon.")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this Digimon's bottom digivolution card, it unsuspends.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Attacking_BT24_074")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
