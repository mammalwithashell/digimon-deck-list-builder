from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_036(CardScript):
    """BT11-036 Chuumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: change_digi_cost
        # Change digivolution cost
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-036 Change digivolution cost")
        effect0.set_effect_description("Change digivolution cost")
        # Reduce digivolution cost by 1 for [Sukamon] name
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If this Digimon has [Sukamon] or [Etemon] in its name, you may play 1 [Chuumon] from your trash suspended without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-036 Play 1 [Chuumon] from trash")
        effect1.set_effect_description("[On Deletion] If this Digimon has [Sukamon] or [Etemon] in its name, you may play 1 [Chuumon] from your trash suspended without paying the cost.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
