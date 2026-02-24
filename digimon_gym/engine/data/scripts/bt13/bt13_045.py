from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_045(CardScript):
    """BT13-045 KingChessmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-045 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Cost -8
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-045 Cost -8")
        effect1.set_effect_description("Cost -8")
        effect1.cost_reduction = 8

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -8"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 8 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By deleting 1 of your other Digimon, you may play 1 Digimon card with [Chessmon] in its name, other than [KingChessmon], from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-045 Delete your 1 Digimon to play 1 Digimon card with [Chessmon] in its name from hand")
        effect2.set_effect_description("[On Play] By deleting 1 of your other Digimon, you may play 1 Digimon card with [Chessmon] in its name, other than [KingChessmon], from your hand without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
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
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Chessmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By deleting 1 of your other Digimon, you may play 1 Digimon card with [Chessmon] in its name, other than [KingChessmon], from your hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-045 Delete your 1 Digimon to play 1 Digimon card with [Chessmon] in its name from hand")
        effect3.set_effect_description("[When Digivolving] By deleting 1 of your other Digimon, you may play 1 Digimon card with [Chessmon] in its name, other than [KingChessmon], from your hand without paying the cost.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Chessmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
