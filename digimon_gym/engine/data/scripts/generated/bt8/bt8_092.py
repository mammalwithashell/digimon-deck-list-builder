from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_092(CardScript):
    """BT8-092 Yuji Musya"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnMove
        # [Your Turn] When one of your Digimon with [X-Antibody] in its traits is moved from your breeding area to your battle area, gain 1 memory and <Draw 1��.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-092 Memory +1 and Draw 1")
        effect0.set_effect_description("[Your Turn] When one of your Digimon with [X-Antibody] in its traits is moved from your breeding area to your battle area, gain 1 memory and <Draw 1��.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
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

        # Timing: EffectTiming.OnAllyAttack
        # [Your Turn] When one of your black Digimon with [X-Antibody] in its traits attacks, you may suspend this Tamer to place 1 card with [X-Antibody] in its traits from your hand under that Digimon as its bottom digivolution card.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-092 Place a Card to digivolution cards")
        effect1.set_effect_description("[Your Turn] When one of your black Digimon with [X-Antibody] in its traits attacks, you may suspend this Tamer to place 1 card with [X-Antibody] in its traits from your hand under that Digimon as its bottom digivolution card.")
        effect1.is_optional = True
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-092 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
