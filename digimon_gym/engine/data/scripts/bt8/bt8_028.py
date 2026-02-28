from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_028(CardScript):
    """BT8-028 CaptainHookmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When your opponent plays a level 5 or higher Digimon, gain 1 memory and <Draw 1>. (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-028 Memory +1 and Draw 1")
        effect0.set_effect_description("[All Turns] When your opponent plays a level 5 or higher Digimon, gain 1 memory and <Draw 1>. (Draw 1 card from your deck.)")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
