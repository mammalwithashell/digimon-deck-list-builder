from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_080(CardScript):
    """Auto-transpiled from DCGO BT24_080.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-080 Also treated as [ChaosGallantmon]")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [Trash] [End of Your Turn] If you have 4 or fewer cards in your hand, 1 of your [Dark Dragon] or [Evil Dragon] trait Digimon may digivolve into this card without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-080 1 [Dark Dragon] or [Evil Dragon] may digivolve into this")
        effect1.set_effect_description("[Trash] [End of Your Turn] If you have 4 or fewer cards in your hand, 1 of your [Dark Dragon] or [Evil Dragon] trait Digimon may digivolve into this card without paying the cost.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass  # TODO: digivolve effect needs card selection

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-080 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-080 Effect")
        effect3.set_effect_description("Effect")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-080 Effect")
        effect4.set_effect_description("Effect")
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Effect
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-080 Effect")
        effect5.set_effect_description("Effect")
        effect5.is_on_deletion = True

        def condition5(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
