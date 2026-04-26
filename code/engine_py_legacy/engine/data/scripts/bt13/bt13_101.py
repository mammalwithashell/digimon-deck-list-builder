from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_101(CardScript):
    """BT13-101 Miki Kurosaki & Megumi Shirakawa"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 Digimon card with [PawnChessmon] in its name from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-101 Play 1 Digimon card with [PawnChessmon] in its name from hand")
        effect0.set_effect_description("[On Play] You may play 1 Digimon card with [PawnChessmon] in its name from your hand without paying the cost.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('PawnChessmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When you play a 2-color black and yellow Digimon, by suspending this Tamer, <Draw 1> and gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-101 Draw 1 and Memory +1")
        effect1.set_effect_description("[All Turns] When you play a 2-color black and yellow Digimon, by suspending this Tamer, <Draw 1> and gain 1 memory.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Tamer must not already be suspended (suspend is cost)
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # The played card must be a 2-color black+yellow Digimon
            played_card = context.get('played_card')
            if not played_card:
                return False
            if not getattr(played_card, 'is_digimon', False):
                return False
            colors = [c.name for c in (getattr(played_card, 'card_colors', []) or [])]
            if not ('Black' in colors and 'Yellow' in colors):
                return False
            # Must be our Digimon (played by same player)
            event_player = context.get('event_player')
            owner = card.owner if card else None
            if event_player and owner and event_player is not owner:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, then Draw 1 and Gain 1 memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()
            # Effect: Draw 1 and gain 1 memory
            player.draw_cards(1)
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-101 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
