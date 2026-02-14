from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_016(CardScript):
    """BT23-016"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-016 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenLinked
        # [Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 1 or fewer Tamers, you may play 1 [Eri Karan] from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-016 Play [Eri Karan] from hand")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 1 or fewer Tamers, you may play 1 [Eri Karan] from your hand without paying the cost.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT23_016_WL")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
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
                player, hand_filter, on_trashed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenLinked
        # [When Linking] <Draw 1>
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-016 Draw 1")
        effect2.set_effect_description("[When Linking] <Draw 1>")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
