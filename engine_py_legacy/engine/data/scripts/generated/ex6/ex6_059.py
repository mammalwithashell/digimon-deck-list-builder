from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_059(CardScript):
    """EX6-059 Barbamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: scapegoat
        # Scapegoat
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-059 Scapegoat")
        effect0.set_effect_description("Scapegoat")
        effect0._is_scapegoat = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash 1 card in your opponent's hand without looking.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-059 Trash 1 card in your opponent's hand")
        effect1.set_effect_description("[On Play] Trash 1 card in your opponent's hand without looking.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash 1 card in your opponent's hand without looking.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-059 Trash 1 card in your opponent's hand")
        effect2.set_effect_description("[When Digivolving] Trash 1 card in your opponent's hand without looking.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
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
        # [All Turns] [Once Per Turn] When a card is trashed from your opponent's hand, you may play 1 purple card with a play cost of 10 or less from your trash without paying the cost. For each card in your opponent's hand, reduce this effect's play cost maximum by 1.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-059 Play 1 purple card with a play cost of 10 or less from your trash")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When a card is trashed from your opponent's hand, you may play 1 purple card with a play cost of 10 or less from your trash without paying the cost. For each card in your opponent's hand, reduce this effect's play cost maximum by 1.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("PlayFromTrash_059")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
