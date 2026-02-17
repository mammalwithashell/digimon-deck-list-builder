from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_097(CardScript):
    """P-097 Zubamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing this card under 1 of your other Digimon in play as its bottom digivolution card, reveal the top 3 cards of your deck. Place those cards at either the top or bottom of your deck in any order. Then, if you have a Digimon with the [Legend-Arms] trait in play, gain 2 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-097 Place this Digimon under your Digimon's digivolution cards to reveal deck tops and to gain Memory +2")
        effect0.set_effect_description("[On Play] By placing this card under 1 of your other Digimon in play as its bottom digivolution card, reveal the top 3 cards of your deck. Place those cards at either the top or bottom of your deck in any order. Then, if you have a Digimon with the [Legend-Arms] trait in play, gain 2 memory.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 2 memory, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Legend-Arms' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 4, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("P-097 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_inherited_effect = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
