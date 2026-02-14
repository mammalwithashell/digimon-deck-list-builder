from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_102(CardScript):
    """BT23-102"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-102 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 5
        effect0._alt_digi_cost = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-102 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Jogress Condition"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DNA/Jogress digivolution condition — handled by engine
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: barrier
        # Barrier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-102 Barrier")
        effect2.set_effect_description("Barrier")
        effect2._is_barrier = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 level 5 or lower yellow or purple card from your hand or trash without paying the cost. Then, if this Digimon's stack has 2 or more same-level cards, trash the top cards of both players' security stacks so that they have 3 cards left.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-102 Play 1 level 5 or lower yellow/purple card from hand or trash. then if digimon has 2+ same level cards in stack, trash both security till 3")
        effect3.set_effect_description("[When Digivolving] You may play 1 level 5 or lower yellow or purple card from your hand or trash without paying the cost. Then, if this Digimon's stack has 2 or more same-level cards, trash the top cards of both players' security stacks so that they have 3 cards left.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] [Once Per Turn] When security stacks are removed from, you may place 1 Digimon as the bottom security card.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-102 Place 1 digimon as bottom security")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When security stacks are removed from, you may place 1 Digimon as the bottom security card.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT23_102_AT")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
