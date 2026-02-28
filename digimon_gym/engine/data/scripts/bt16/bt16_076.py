from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_076(CardScript):
    """BT16-076 Soloogarmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-076 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 2 cards in your hand, delete 1 of your opponent's Digimon with 6000 DP or less. If this effect didn't delete, you may play 1 level 4 or lower card with the [SoC] trait from your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-076 Delete an opponent's Digimon with 6000 DP or less, if the effect didn't delete, play a level 4 or lower [SoC] card.")
        effect1.set_effect_description("[When Digivolving] By trashing 2 cards in your hand, delete 1 of your opponent's Digimon with 6000 DP or less. If this effect didn't delete, you may play 1 level 4 or lower card with the [SoC] trait from your trash without paying the cost.")
        effect1.is_optional = True
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
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if getattr(c, 'level', None) is None or c.level < 2:
                    return False
                if not (any('SoC' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if getattr(c, 'level', None) is None or c.level < 2:
                    return False
                if not (any('SoC' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] When one of your other Digimon with the [SoC] trait is deleted, this Digimon with a Tamer with the [SoC] trait in its digivolution cards may digivolve into [Fenriloogamon] from your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-076 Digivolve into [Fenriloogamon] from your trash.")
        effect2.set_effect_description("[All Turns] When one of your other Digimon with the [SoC] trait is deleted, this Digimon with a Tamer with the [SoC] trait in its digivolution cards may digivolve into [Fenriloogamon] from your trash without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if getattr(c, 'level', None) is None or c.level < 2:
                    return False
                if not (any('SoC' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] [Once Per Turn] If your opponent has 1 or more memory, unsuspend this Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT16-076 Unsuspend this Digimon.")
        effect3.set_effect_description("[End of Attack] [Once Per Turn] If your opponent has 1 or more memory, unsuspend this Digimon.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Unsuspend_BT16_076")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
