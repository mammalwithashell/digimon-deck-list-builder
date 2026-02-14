from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_015(CardScript):
    """BT20-015 Hisyaryumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-015 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Ginryumon] with [Chronicle] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Ginryumon"
        effect0._alt_digi_trait = "Chronicle"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Ginryumon'))):
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Chronicle' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 [Dorumon]/[Ryudamon] from your hand to your empty breeding area without paying the cost. Then if during an attack, until the end of your opponent's turn, 1 of your Digimon gains <Security Attack +1> and gets +5000 DP.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-015 Play 1 Digimon from your hand to your empty breeding area")
        effect1.set_effect_description("[On Play] You may play 1 [Dorumon]/[Ryudamon] from your hand to your empty breeding area without paying the cost. Then if during an attack, until the end of your opponent's turn, 1 of your Digimon gains <Security Attack +1> and gets +5000 DP.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +5000, Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(5000)
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
                player, hand_filter, on_trashed, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 [Dorumon]/[Ryudamon] from your hand to your empty breeding area without paying the cost. Then if during an attack, until the end of your opponent's turn, 1 of your Digimon gains <Security Attack +1> and gets +5000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-015 Play 1 Digimon from your hand to your empty breeding area")
        effect2.set_effect_description("[When Digivolving] You may play 1 [Dorumon]/[Ryudamon] from your hand to your empty breeding area without paying the cost. Then if during an attack, until the end of your opponent's turn, 1 of your Digimon gains <Security Attack +1> and gets +5000 DP.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP +5000, Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(5000)
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
                player, hand_filter, on_trashed, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-015 Ignore Security Effect")
        effect3.set_effect_description("Effect")
        effect3.is_inherited_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
