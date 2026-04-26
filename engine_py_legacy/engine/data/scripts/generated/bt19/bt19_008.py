from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_008(CardScript):
    """BT19-008 Shoutmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-008 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Xros Heart] trait for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_trait = "Xros Heart"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] This Digimon may digivolve into [OmniShoutmon] under your Tamers without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-008 Digivolve into [OmniShoutmon] from under your Tamers")
        effect1.set_effect_description("[On Play] This Digimon may digivolve into [OmniShoutmon] under your Tamers without paying the cost.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Reveal the top 3 cards of your deck. Play 1 Tamer card with the [Xros Heart] trait among them without paying the cost. Return the rest to the bottom of the deck. Then, <Save>.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-008 Reveal top 3 cards of the deck and play 1 [Xros Heart] Tamer from them, then <Save>")
        effect2.set_effect_description("[On Deletion] Reveal the top 3 cards of your deck. Play 1 Tamer card with the [Xros Heart] trait among them without paying the cost. Return the rest to the bottom of the deck. Then, <Save>.")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: rush
        # Rush
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-008 Rush")
        effect3.set_effect_description("Rush")
        effect3.is_inherited_effect = True
        effect3._is_rush = True

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
