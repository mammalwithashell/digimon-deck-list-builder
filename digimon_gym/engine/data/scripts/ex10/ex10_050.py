from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_050(CardScript):
    """EX10-050 Baalmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-050 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 3 cards of your deck. Then, if you have 5 or more cards in your trash, this Digimon gains <Reboot> (Unsuspend this Digimon during your opponent's unsuspend phase.) and <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-050 Trash top 3 cards of deck, then if you have 5 or more in trash, gain Reboot and Blocker until opponent turn ends")
        effect1.set_effect_description("[On Play] Trash the top 3 cards of your deck. Then, if you have 5 or more cards in your trash, this Digimon gains <Reboot> (Unsuspend this Digimon during your opponent's unsuspend phase.) and <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.")
        effect1.is_on_play = True
        effect1._is_reboot = True
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Reboot, Gain Keyword Blocker, Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_reboot')
                perm.grant_keyword('_is_blocker')
            # Mill 3 cards from own deck
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash the top 3 cards of your deck. Then, if you have 5 or more cards in your trash, this Digimon gains <Reboot> (Unsuspend this Digimon during your opponent's unsuspend phase.) and <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-050 Trash top 3 cards of deck, then if you have 5 or more in trash, gain Reboot and Blocker until opponent turn ends")
        effect2.set_effect_description("[When Digivolving] Trash the top 3 cards of your deck. Then, if you have 5 or more cards in your trash, this Digimon gains <Reboot> (Unsuspend this Digimon during your opponent's unsuspend phase.) and <Blocker> (At blocker timing, by suspending this Digimon, it becomes the attack target.) until your opponent's turn ends.")
        effect2.is_when_digivolving = True
        effect2._is_reboot = True
        effect2._is_blocker = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Reboot, Gain Keyword Blocker, Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_reboot')
                perm.grant_keyword('_is_blocker')
            # Mill 3 cards from own deck
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If you have 10 or more cards in your trash, you may play 1 [Beelzemon] from your trash without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-050 Play 1 [Beelzemon] from trash")
        effect3.set_effect_description("[On Deletion] If you have 10 or more cards in your trash, you may play 1 [Beelzemon] from your trash without paying the cost.")
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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

        # Factory effect: dp_modifier
        # DP modifier
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-050 DP modifier")
        effect4.set_effect_description("DP modifier")
        effect4.is_inherited_effect = True
        effect4.dp_modifier = 0

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
