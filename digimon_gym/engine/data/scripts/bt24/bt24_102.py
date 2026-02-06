from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_102(CardScript):
    """Auto-transpiled from DCGO BT24_102.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Gain 1 memory. Then, if you have 5 or more memory, suspend this Tamer and ＜Draw 1＞.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-102 Gain 1 Memory. If 5+ Memory, suspend and Draw 1.")
        effect0.set_effect_description("[Start of Your Main Phase] Gain 1 memory. Then, if you have 5 or more memory, suspend this Tamer and ＜Draw 1＞.")

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
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)
            # Suspend opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                target.suspend()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] By suspending this Tamer, you may activate 1 [On Play] or [When Digivolving] effect of 1 of your [Olympos XII] trait Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-102 Suspend this tamer to use an On Play or When Digivolving")
        effect1.set_effect_description("[End of Your Turn] By suspending this Tamer, you may activate 1 [On Play] or [When Digivolving] effect of 1 of your [Olympos XII] trait Digimon.")
        effect1.is_optional = True

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
            # Suspend opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                target.suspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-102 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
