from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_052(CardScript):
    """BT22-052 Leopardmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-052 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-052 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 5000 DP or lower Digimon card from your hand without paying the cost. Then, all of your level 3 or higher Digimon gain <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-052 Play 1 Digimon from hand, Then gain blocker")
        effect2.set_effect_description("[On Play] You may play 1 5000 DP or lower Digimon card from your hand without paying the cost. Then, all of your level 3 or higher Digimon gain <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 5000 DP or lower Digimon card from your hand without paying the cost. Then, all of your level 3 or higher Digimon gain <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-052 Play 1 Digimon from hand, Then gain blocker")
        effect3.set_effect_description("[When Digivolving] You may play 1 5000 DP or lower Digimon card from your hand without paying the cost. Then, all of your level 3 or higher Digimon gain <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your other Digimon would leave the battle area, gain 2 memory.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT22-052 Memory +2")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any of your other Digimon would leave the battle area, gain 2 memory.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("AllTurn_BT22-052")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
