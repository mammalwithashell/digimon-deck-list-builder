from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_010(CardScript):
    """BT13-010 Biyomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If played by an effect, by returning 1 of your [Kristy Damon]s to the hand, this Digimon may digivolve into [Garudamon] in the hand, ignoring its digivolution requirements and without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-010 Return 1 [Kristy Damon] to hand and this Digimon digivolves to [Garudamon]")
        effect0.set_effect_description("[On Play] If played by an effect, by returning 1 of your [Kristy Damon]s to the hand, this Digimon may digivolve into [Garudamon] in the hand, ignoring its digivolution requirements and without paying the cost.")
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
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] <Draw 1> (Draw 1 card from your deck.)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-010 Draw 1")
        effect1.set_effect_description("[On Deletion] <Draw 1> (Draw 1 card from your deck.)")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
