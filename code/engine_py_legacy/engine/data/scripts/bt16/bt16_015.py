from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_015(CardScript):
    """BT16-015 Phoenixmon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-015 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Phoenixmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blitz
        # Blitz
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-015 Blitz")
        effect1.set_effect_description("Blitz")
        effect1.is_on_play = True
        effect1._is_blitz = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 11000 DP or lower red Digimon card with [Avian], [Bird], [Beast], [Animal], or [Sovereign], other than [Sea Animal] in one of its traits from your hand without paying the cost. Delete 1 of your opponent's Digimon with as much or less DP as the Digimon this effect played.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("BT16-015 Play 1 digimon, delete 1 digimon")
        effect2.set_effect_description("[On Deletion] You may play 1 11000 DP or lower red Digimon card with [Avian], [Bird], [Beast], [Animal], or [Sovereign], other than [Sea Animal] in one of its traits from your hand without paying the cost. Delete 1 of your opponent's Digimon with as much or less DP as the Digimon this effect played.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
