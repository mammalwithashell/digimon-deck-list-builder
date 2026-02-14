from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_030(CardScript):
    """BT23-030"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-030 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-030 Alliance")
        effect1.set_effect_description("Alliance")
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] [Once Per Turn] By paying 1 cost, you may play 1 play cost 3 or lower card with [Chuumon] or [Sukamon] in its name or the [CS] trait from your hand without paying the cost. Then, 1 of your level 3 or higher Digimon gains <Reboot> and <Blocker> until your opponent's turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-030 By paying 1 cost, play 3 cost or lower [Chuumon]/[Sukamon] in name /[CS] trait from your hand, then 1 level 3+ digimon gains <Reboot> and <Blocker>")
        effect2.set_effect_description("[Main] [Once Per Turn] By paying 1 cost, you may play 1 play cost 3 or lower card with [Chuumon] or [Sukamon] in its name or the [CS] trait from your hand without paying the cost. Then, 1 of your level 3 or higher Digimon gains <Reboot> and <Blocker> until your opponent's turn ends.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT23_030_Main")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
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
                if not (any('Chuumon' in _n or 'Sukamon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                if getattr(c, 'level', None) is None or c.level < 3:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Chuumon' in _n or 'Sukamon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                if getattr(c, 'level', None) is None or c.level < 3:
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

        # Factory effect: alliance
        # Alliance
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-030 Alliance")
        effect3.set_effect_description("Alliance")
        effect3.is_inherited_effect = True
        effect3._is_alliance = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
