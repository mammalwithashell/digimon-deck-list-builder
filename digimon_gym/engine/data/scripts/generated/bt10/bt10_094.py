from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_094(CardScript):
    """BT10-094 Breaclaw"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your Digimon gets +2000 DP for the turn. Then, by placing 1 Digimon card with [Gammamon] in its name from your hand under 1 of your Digimon as its bottom digivolution card, <Draw 1>. (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-094 Draw 1, DP +2000")
        effect0.set_effect_description("[Main] 1 of your Digimon gets +2000 DP for the turn. Then, by placing 1 Digimon card with [Gammamon] in its name from your hand under 1 of your Digimon as its bottom digivolution card, <Draw 1>. (Draw 1 card from your deck.)")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, DP +2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if perm:
                perm.change_dp(2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Gammamon] from your hand or trash without payings its memory cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-094 Play Card")
        effect1.set_effect_description("[Security] You may play 1 [Gammamon] from your hand or trash without payings its memory cost.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
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

        return effects
