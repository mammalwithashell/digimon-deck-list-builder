from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_215(CardScript):
    """P-215 Icemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-215 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Ice-Snow] trait for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_trait = "Ice-Snow"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Ice-Snow' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnMove
        # Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck
        effect1 = ICardEffect()
        effect1.set_effect_name("P-215 Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck")
        effect1.set_effect_description("Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck")
        effect1._is_cannot_return_to_hand = True
        effect1._is_cannot_return_to_deck = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck"""
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
            if perm:
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck
        effect2 = ICardEffect()
        effect2.set_effect_name("P-215 Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck")
        effect2.set_effect_description("Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck")
        effect2.is_on_play = True
        effect2._is_cannot_return_to_hand = True
        effect2._is_cannot_return_to_deck = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck"""
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
            if perm:
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck
        effect3 = ICardEffect()
        effect3.set_effect_name("P-215 Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck")
        effect3.set_effect_description("Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck")
        effect3.is_when_digivolving = True
        effect3._is_cannot_return_to_hand = True
        effect3._is_cannot_return_to_deck = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck"""
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
            if perm:
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: blocker
        # Blocker
        effect4 = ICardEffect()
        effect4.set_effect_name("P-215 Blocker")
        effect4.set_effect_description("Blocker")
        effect4.is_inherited_effect = True
        effect4._is_blocker = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
