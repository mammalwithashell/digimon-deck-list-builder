from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_016(CardScript):
    """BT19-016 Gaossmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 [Blue Flare] trait Digimon card from your hand under any of your Tamers, <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-016 Place 1 [Blue Flare] card under 1 of your Tamers to <Draw 1>")
        effect0.set_effect_description("[On Play] By placing 1 [Blue Flare] trait Digimon card from your hand under any of your Tamers, <Draw 1>.")
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
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] By placing 1 [Blue Flare] trait Digimon card from your hand under any of your Tamers, <Draw 1>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-016 Place 1 [Blue Flare] card from hand under 1 of your Tamers to <Draw 1>")
        effect1.set_effect_description("[On Deletion] By placing 1 [Blue Flare] trait Digimon card from your hand under any of your Tamers, <Draw 1>.")
        effect1.is_optional = True
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
