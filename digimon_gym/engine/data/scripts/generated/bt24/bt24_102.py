from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_102(CardScript):
    """BT24-102 Homeros"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Gain 1 memory. Then, if you have 5 or more memory, suspend this Tamer and ＜Draw 1＞.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-102 Gain 1 Memory. If 5+ Memory, suspend and Draw 1.")
        effect0.set_effect_description("[Start of Your Main Phase] Gain 1 memory. Then, if you have 5 or more memory, suspend this Tamer and ＜Draw 1＞.")

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
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-102 All your Digimon DP modifier")
        effect1.set_effect_description("All your Digimon DP modifier")
        effect1.dp_modifier = 1000
        effect1._applies_to_all_own_digimon = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] By suspending this Tamer, you may activate 1 [On Play] or [When Digivolving] effect of 1 of your [Olympos XII] trait Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-102 Suspend this tamer to use an On Play or When Digivolving")
        effect2.set_effect_description("[End of Your Turn] By suspending this Tamer, you may activate 1 [On Play] or [When Digivolving] effect of 1 of your [Olympos XII] trait Digimon.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-102 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
