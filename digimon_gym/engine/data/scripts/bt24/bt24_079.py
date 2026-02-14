from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_079(CardScript):
    """BT24-079 Hadesmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-079 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: overclock
        # Overclock
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-079 Overclock")
        effect1.set_effect_description("Overclock")
        effect1._is_overclock = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Play Card, Trash From Hand
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-079 Play a lvl 4- from trash. Add new link to a digimon")
        effect2.set_effect_description("Play Card, Trash From Hand")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Play Card, Trash From Hand
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-079 Play a lvl 4- from trash. Add new link to a digimon")
        effect3.set_effect_description("Play Card, Trash From Hand")
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
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
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

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When other Digimon are deleted, you may activate 1 of this Digimon's [When Digivolving] effects.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-079 Activate [When Digivolving]")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When other Digimon are deleted, you may activate 1 of this Digimon's [When Digivolving] effects.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_079_AT_Activate_WD")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
