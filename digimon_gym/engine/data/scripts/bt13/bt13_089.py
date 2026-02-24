from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_089(CardScript):
    """BT13-089 Ravemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        #  [End of Your Turn] By deleting this Digimon that has a digivolution card with [Bird] or [Avian] in one of its traits, at the end of your opponent's turn, you may play 1 [Ravemon] from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-089 Delete this Digimon to play 1 [Ravemon] from trash at the end of opponent's turn")
        effect0.set_effect_description(" [End of Your Turn] By deleting this Digimon that has a digivolution card with [Bird] or [Avian] in one of its traits, at the end of your opponent's turn, you may play 1 [Ravemon] from your trash without paying the cost.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # Play Card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-089 Play 1 [Ravemon] from trash")
        effect1.set_effect_description("Play Card")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
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
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 [Falcomon] or [Keenan Crier] from your hand or trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-089 Play 1 [Falcomon] or [Keenan Crier] from hand or trash")
        effect2.set_effect_description("[On Deletion] You may play 1 [Falcomon] or [Keenan Crier] from your hand or trash without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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

        return effects
