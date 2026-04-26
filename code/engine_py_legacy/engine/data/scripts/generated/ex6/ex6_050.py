from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_050(CardScript):
    """EX6-050 Feresmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-050 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If your opponent has 5 or fewer cards in their hand, gain 1 memory. If your opponent has 7 or more cards in their hand, your opponent trashes 1 card in their hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-050 Gain 1 memory/Opponent trashes 1 card")
        effect1.set_effect_description("[When Digivolving] If your opponent has 5 or fewer cards in their hand, gain 1 memory. If your opponent has 7 or more cards in their hand, your opponent trashes 1 card in their hand.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
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

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If your opponent has 5 or fewer cards in their hand, gain 1 memory. If your opponent has 7 or more cards in their hand, your opponent trashes 1 card in their hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-050 Gain 1 memory/Opponent trashes 1 card")
        effect2.set_effect_description("[On Deletion] If your opponent has 5 or fewer cards in their hand, gain 1 memory. If your opponent has 7 or more cards in their hand, your opponent trashes 1 card in their hand.")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
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

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Your opponent may trash 1 card in their hand. If they don't, you may play 1 purple level 3 Digimon card from your trash wihtout paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-050 Your opponent trashes 1 card/Play 1 level 3 purple Digimon from your trash.")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] Your opponent may trash 1 card in their hand. If they don't, you may play 1 purple level 3 Digimon card from your trash wihtout paying the cost.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("TrashOrPlay_EX6_050")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
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

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
