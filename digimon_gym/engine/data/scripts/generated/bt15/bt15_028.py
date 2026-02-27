from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_028(CardScript):
    """BT15-028 Divermon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the bottom 3 digivolution cards of 1 of your opponent's Digimon. Then, if your opponent has no Digimon with digivolution cards, you may play 1 blue Tamer card from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-028 Trash digivolution cards and play 1 Tamer from hand")
        effect0.set_effect_description("[On Play] Trash the bottom 3 digivolution cards of 1 of your opponent's Digimon. Then, if your opponent has no Digimon with digivolution cards, you may play 1 blue Tamer card from your hand without paying the cost.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 0):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                if not ('Blue' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
