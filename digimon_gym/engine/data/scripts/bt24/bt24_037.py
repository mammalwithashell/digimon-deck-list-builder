from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_037(CardScript):
    """Auto-transpiled from DCGO BT24_037.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-037 Effect")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-037 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-037 Effect")
        effect2.set_effect_description("Effect")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower yellow, red or [TS] trait Digimon card from its digivolution cards without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-037 Play 1 level 4- [Yellow]/[Red]/[TS] trait digimon from digivolution sources")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower yellow, red or [TS] trait Digimon card from its digivolution cards without paying the cost.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_037_AT")

        def condition3(context: Dict[str, Any]) -> bool:
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

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower yellow, red or [TS] trait Digimon card from its digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-037 Play 1 level 4- [Yellow]/[Red]/[CS] trait digimon from digivolution sources")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower yellow, red or [TS] trait Digimon card from its digivolution cards without paying the cost.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_037_AT_ESS")

        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
