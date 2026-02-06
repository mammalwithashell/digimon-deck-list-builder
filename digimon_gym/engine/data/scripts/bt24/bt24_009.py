from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_009(CardScript):
    """Auto-transpiled from DCGO BT24_009.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing 1 card with the [Demon], [Shaman] or [Titan] trait from your hand, <Draw 2>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-009 By trashing 1 card, draw 2")
        effect0.set_effect_description("[On Play] By trashing 1 card with the [Demon], [Shaman] or [Titan] trait from your hand, <Draw 2>.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 2, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(2)
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn] [Once Per Turn] When your hand is trashed from, this [Demon] or [Titan] trait Digimon may digivolve into [Titamon] or a [Titan] trait Digimon card in the trash with the digivolution cost reduced by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-009 When your hand is trashed from, digivolve")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When your hand is trashed from, this [Demon] or [Titan] trait Digimon may digivolve into [Titamon] or a [Titan] trait Digimon card in the trash with the digivolution cost reduced by 1.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_009_YT_ESS")

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass  # TODO: digivolve effect needs card selection

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
