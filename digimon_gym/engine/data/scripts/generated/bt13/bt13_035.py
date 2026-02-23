from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_035(CardScript):
    """BT13-035 PawnChessmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If it's your turn, you may play 1 level 3 or lower Digimon card with [Chessmon] in its name from your hand without paying the cost. If you have 8 or more Digimon cards with [Chessmon] in their names in your trash, add 2 to the maximum level of the card this effect can play.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-035 Play 1 Digimon card with [Chessmon] in its name from hand")
        effect0.set_effect_description("[On Deletion] If it's your turn, you may play 1 level 3 or lower Digimon card with [Chessmon] in its name from your hand without paying the cost. If you have 8 or more Digimon cards with [Chessmon] in their names in your trash, add 2 to the maximum level of the card this effect can play.")
        effect0.is_optional = True
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on deletion — validated by engine timing
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
                if not (any('Chessmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: reboot
        # Reboot
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-035 Reboot")
        effect1.set_effect_description("Reboot")
        effect1.is_inherited_effect = True
        effect1._is_reboot = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
