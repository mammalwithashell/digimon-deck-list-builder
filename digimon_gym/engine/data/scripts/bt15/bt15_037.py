from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_037(CardScript):
    """BT15-037 Gatomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: barrier
        # Barrier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-037 Barrier")
        effect0.set_effect_description("Barrier")
        effect0._is_barrier = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: barrier
        # Barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-037 Barrier")
        effect1.set_effect_description("Barrier")
        effect1.is_inherited_effect = True
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns][Once per turn] When a card is removed from your security stack, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnLoseSecurity)
        effect2.set_effect_name("BT15-037 Gain 1 memory.")
        effect2.set_effect_description("[All Turns][Once per turn] When a card is removed from your security stack, gain 1 memory.")
        effect2.set_max_count_per_turn(1)

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDiscardSecurity
        # When an effect trashes this card from your security stack, you may play this card without paying the cost.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDiscardSecurity)
        effect3.set_effect_name("BT15-037 Play this card without paying the cost")
        effect3.set_effect_description("When an effect trashes this card from your security stack, you may play this card without paying the cost.")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
