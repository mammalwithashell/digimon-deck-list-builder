from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_051(CardScript):
    """Auto-transpiled from DCGO BT24_051.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if there are 3 or more Digimon, reduce the play cost by 5.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-051 Reduce play cost (5)")
        effect0.set_effect_description("When this card would be played, if there are 3 or more Digimon, reduce the play cost by 5.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-051 Play Cost -5")
        effect1.set_effect_description("Effect")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-051 Effect")
        effect2.set_effect_description("Effect")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-051 Effect")
        effect3.set_effect_description("Effect")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-051 Effect")
        effect4.set_effect_description("Effect")
        effect4.set_hash_string("BT24_051_WD_WA")
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # Effect
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-051 Effect")
        effect5.set_effect_description("Effect")
        effect5.set_hash_string("BT24_051_WD_WA")
        effect5.is_on_attack = True

        def condition5(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Timing: EffectTiming.None
        # Effect
        effect6 = ICardEffect()
        effect6.set_effect_name("BT24-051 [Your Turn] All of your [Iliad] trait Digimon gain <Rush> and <Piercing>.")
        effect6.set_effect_description("Effect")

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
