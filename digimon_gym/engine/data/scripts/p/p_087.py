from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_087(CardScript):
    """P-087 Ritsu Kodo"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When you would play a [Pulsemon], by suspending this tamer, if you have 3 or more security cards, <Draw 1> (Draw 1 card from your deck). If you have 3 or fewer security cards, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-087 Draw 1 and Memory +1")
        effect0.set_effect_description("[Your Turn] When you would play a [Pulsemon], by suspending this tamer, if you have 3 or more security cards, <Draw 1> (Draw 1 card from your deck). If you have 3 or fewer security cards, gain 1 memory.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("P-087 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
