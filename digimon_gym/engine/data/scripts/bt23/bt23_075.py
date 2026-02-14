from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_075(CardScript):
    """BT23-075"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-075 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Eater Legion] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Eater Legion"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Eater Legion'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] [When Digivolving] Return 1 of your opponent's play cost 6 or lower Digimon or Tamers to the bottom of the deck. For each of your [Mother Eater]'s digivolution cards in the breeding area, add 1 to this effect's play cost maximum.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-075 Return 1 of your opponent's Digimon or Tamers to the bottom of the deck.")
        effect1.set_effect_description("[On Play] [When Digivolving] Return 1 of your opponent's play cost 6 or lower Digimon or Tamers to the bottom of the deck. For each of your [Mother Eater]'s digivolution cards in the breeding area, add 1 to this effect's play cost maximum.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother Eater'))):
                return False
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
        # [When Digivolving] Return 1 of your opponent's play cost 6 or lower Digimon or Tamers to the bottom of the deck. For each of your [Mother Eater]'s digivolution cards in the breeding area, add 1 to this effect's play cost maximum.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-075 Return 1 of your opponent's Digimon or Tamers to the bottom of the deck.")
        effect2.set_effect_description("[When Digivolving] Return 1 of your opponent's play cost 6 or lower Digimon or Tamers to the bottom of the deck. For each of your [Mother Eater]'s digivolution cards in the breeding area, add 1 to this effect's play cost maximum.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother Eater'))):
                return False
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

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area other than by your effects, you may play 1 [Eater] trait Digimon card from your hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-075 Play 1 [Eater] digimon from hand")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area other than by your effects, you may play 1 [Eater] trait Digimon card from your hand without paying the cost.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEndTurn
        # [End of Opponent's Turn] [Once Per Turn] Delete 1 of your opponent's lowest play cost Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-075 Delete 1 Digimon with the lowest play cost")
        effect4.set_effect_description("[End of Opponent's Turn] [Once Per Turn] Delete 1 of your opponent's lowest play cost Digimon.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Delete_BT23_075")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
