from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_020(CardScript):
    """BT19-020 Greymon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: rush
        # Rush
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-020 Rush")
        effect0.set_effect_description("Rush")
        effect0._is_rush = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If you have 1 or fewer Tamers, you may play 1 [Kiriha Aonuma] from your hand without paying the cost. Then, <Save>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-020 Play 1 Tamer with [Kiriha Aonuma] in its name from hand, then <Save>")
        effect1.set_effect_description("[On Deletion] If you have 1 or fewer Tamers, you may play 1 [Kiriha Aonuma] from your hand without paying the cost. Then, <Save>.")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: reboot
        # Reboot
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-020 Reboot")
        effect2.set_effect_description("Reboot")
        effect2.is_inherited_effect = True
        effect2._is_reboot = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
