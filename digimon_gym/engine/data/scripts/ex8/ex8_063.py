from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_063(CardScript):
    """EX8-063 Barbamon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-063 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Barbamon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] Your opponent may trash 1 card in their hand. If this effect didn't trash, you may play 1 [Fallen Angel] trait Digimon card with a play cost of 7 or less from your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX8-063 Opponent trashes 1 card, or you play a fallen angel")
        effect1.set_effect_description("[When Digivolving] [Once Per Turn] Your opponent may trash 1 card in their hand. If this effect didn't trash, you may play 1 [Fallen Angel] trait Digimon card with a play cost of 7 or less from your trash without paying the cost.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashOrPlay_EX8_063")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Your opponent may trash 1 card in their hand. If this effect didn't trash, you may play 1 [Fallen Angel] trait Digimon card with a play cost of 7 or less from your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("EX8-063 Opponent trashes 1 card, or you play a fallen angel")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] Your opponent may trash 1 card in their hand. If this effect didn't trash, you may play 1 [Fallen Angel] trait Digimon card with a play cost of 7 or less from your trash without paying the cost.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("TrashOrPlay_EX8_063")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDiscardHand
        # [All Turns] [Once Per Turn] When your opponent's hand is trashed from, if [Barbamon] or [X Antibody] is in this Digimon's digivolution cards, trash their top security card.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDiscardHand)
        effect3.set_effect_name("EX8-063 Trash opponent's security")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When your opponent's hand is trashed from, if [Barbamon] or [X Antibody] is in this Digimon's digivolution cards, trash their top security card.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("TrashSecurity_EX8_063")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
