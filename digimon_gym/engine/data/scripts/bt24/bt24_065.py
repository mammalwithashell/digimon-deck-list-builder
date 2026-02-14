from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_065(CardScript):
    """BT24-065 Diaboromon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-065 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Diaboromon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Diaboromon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Diaboromon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: overclock
        # Overclock
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-065 Overclock")
        effect1.set_effect_description("Overclock")
        effect1._is_overclock = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-065 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Delete, De Digivolve
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-065 De-digivolve 1 opponent's Digimon. Delete all their highest play cost Digimon.")
        effect3.set_effect_description("Delete, De Digivolve")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete, De Digivolve"""
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
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your Digimon with [Diaboromon] in their names would leave the battle area, you may play 1 [Diaboromon] from your hand or this Digimon's digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-065 Play a Diaboromon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any of your Digimon with [Diaboromon] in their names would leave the battle area, you may play 1 [Diaboromon] from your hand or this Digimon's digivolution cards without paying the cost.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_65_AT_Play_Diaboromon")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Diaboromon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Diaboromon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
