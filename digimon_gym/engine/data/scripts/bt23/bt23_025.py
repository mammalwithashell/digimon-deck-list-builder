from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_025(CardScript):
    """BT23-025"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-025 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Hand] [Main] If you have a Digimon or Tamer with the [CS] trait, by paying 5 cost, give 3 of your opponent's Digimon <Security A. -1> until their turn ends. Then, place this card as the top security card.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-025 Give 3 Digimon Sec-1")
        effect1.set_effect_description("[Hand] [Main] If you have a Digimon or Tamer with the [CS] trait, by paying 5 cost, give 3 of your opponent's Digimon <Security A. -1> until their turn ends. Then, place this card as the top security card.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return 1 of your opponent's Digimon with the lowest level to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-025 Return 1 of your opponent's Digimon to hand")
        effect2.set_effect_description("[On Play] Return 1 of your opponent's Digimon with the lowest level to the hand.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return 1 of your opponent's Digimon with the lowest level to the hand.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-025 Return 1 of your opponent's Digimon to hand")
        effect3.set_effect_description("[When Digivolving] Return 1 of your opponent's Digimon with the lowest level to the hand.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: security_play
        # Security: Play this card
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-025 Security: Play this card")
        effect4.set_effect_description("Security: Play this card")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
