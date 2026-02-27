from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_102(CardScript):
    """BT22-102 Sayo"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # Gain 1 memory (Tamer)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-102 Gain 1 memory (Tamer)")
        effect0.set_effect_description("Gain 1 memory (Tamer)")
        # [Start of Main] Gain 1 memory if opponent has Digimon

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [Your Turn] When one of your Digimon with 2 or more same-level cards in its stack attacks, by suspending this Tamer, digivolve it into a Digimon card with the [Night Claw], [Light Fang], [Galaxy] or [CS] trait in the trash with the digivolution cost reduced by 2.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-102 Digivolve for a reduced cost")
        effect1.set_effect_description("[Your Turn] When one of your Digimon with 2 or more same-level cards in its stack attacks, by suspending this Tamer, digivolve it into a Digimon card with the [Night Claw], [Light Fang], [Galaxy] or [CS] trait in the trash with the digivolution cost reduced by 2.")
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
            """Action: Suspend, Digivolve"""
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
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-102 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
