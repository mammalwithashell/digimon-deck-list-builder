from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_024(CardScript):
    """BT20-024 Seadramon (X Antibody) | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-024 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Seadramon] for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Seadramon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Seadramon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return 1 of your opponent's level 3 Digimon to the bottom of the deck. Then, if [Seadramon]/[X Antibody] is in this Digimon's digivolution cards, 1 of your opponent's Tamers can't suspend until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-024 Return 1 opponent's Digimon to bottom deck and apply an effect to an opponent's Tamer")
        effect1.set_effect_description("[On Play] Return 1 of your opponent's level 3 Digimon to the bottom of the deck. Then, if [Seadramon]/[X Antibody] is in this Digimon's digivolution cards, 1 of your opponent's Tamers can't suspend until the end of their turn.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return 1 of your opponent's level 3 Digimon to the bottom of the deck. Then, if [Seadramon]/[X Antibody] is in this Digimon's digivolution cards, 1 of your opponent's Tamers can't suspend until the end of their turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-024 Return 1 opponent's Digimon to bottom deck and apply an effect to an opponent's Tamer")
        effect2.set_effect_description("[When Digivolving] Return 1 of your opponent's level 3 Digimon to the bottom of the deck. Then, if [Seadramon]/[X Antibody] is in this Digimon's digivolution cards, 1 of your opponent's Tamers can't suspend until the end of their turn.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] If you have 7 or fewer cards in your hand, <Draw 1>.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-024 Draw 1 card")
        effect3.set_effect_description("[When Attacking][Once Per Turn] If you have 7 or fewer cards in your hand, <Draw 1>.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Draw_BT20_024")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
