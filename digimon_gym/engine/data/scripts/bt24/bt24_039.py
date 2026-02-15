from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_039(CardScript):
    """BT24-039 Piximon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-039 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-039 Ignore Battle")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] If your opponent has a level 6 or higher Digimon, play this card without battling and without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-039 Play this card without battling")
        effect2.set_effect_description("[Security] If your opponent has a level 6 or higher Digimon, play this card without battling and without paying the cost.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-039 Blocker")
        effect3.set_effect_description("Blocker")
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: barrier
        # Barrier
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-039 Barrier")
        effect4.set_effect_description("Barrier")
        effect4._is_barrier = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Trigger <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-039 Recovery +1 (Deck)")
        effect5.set_effect_description("[On Deletion] Trigger <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)")
        effect5.is_inherited_effect = True
        effect5.is_on_deletion = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
