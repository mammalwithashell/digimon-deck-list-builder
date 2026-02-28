from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_094(CardScript):
    """BT8-094 Digimon Emperor"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] When one of your opponent's level 5 or lower Digimon is deleted, you may suspend this Tamer to <Draw 1��. (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-094 Draw 1")
        effect0.set_effect_description("[All Turns] When one of your opponent's level 5 or lower Digimon is deleted, you may suspend this Tamer to <Draw 1��. (Draw 1 card from your deck.)")
        effect0.is_optional = True
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnMove
        # [Opponent's Turn] When one of your opponent's level 3 Digimon is moved from their breeding area to their battle area, gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-094 Memory +2")
        effect1.set_effect_description("[Opponent's Turn] When one of your opponent's level 3 Digimon is moved from their breeding area to their battle area, gain 2 memory.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-094 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
