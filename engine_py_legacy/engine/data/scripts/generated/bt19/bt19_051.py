from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_051(CardScript):
    """BT19-051 AtlurBallistamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-051 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Xros Heart] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "Xros Heart"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-051 Also treated as [Ballistamon] for a DigiXros")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your Digimon can't be returned to the hand or deck and gets +3000 DP until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-051 1 of your Digimon can't be returned to the hand or deck and gets +3000 DP")
        effect2.set_effect_description("[On Play] 1 of your Digimon can't be returned to the hand or deck and gets +3000 DP until the end of your opponent's turn.")
        effect2.set_max_count_per_turn(1)
        effect2.is_on_play = True
        effect2._is_cannot_return_to_hand = True
        effect2._is_cannot_return_to_deck = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP +3000, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck, Grant Bounce Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if perm:
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')
            # Prevent return to hand/deck via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_RETURNED, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your Digimon can't be returned to the hand or deck and gets +3000 DP until the end of your opponent's turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-051 1 of your Digimon can't be returned to the hand or deck and gets +3000 DP")
        effect3.set_effect_description("[When Digivolving] 1 of your Digimon can't be returned to the hand or deck and gets +3000 DP until the end of your opponent's turn.")
        effect3.set_max_count_per_turn(1)
        effect3.is_when_digivolving = True
        effect3._is_cannot_return_to_hand = True
        effect3._is_cannot_return_to_deck = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP +3000, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck, Grant Bounce Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if perm:
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')
            # Prevent return to hand/deck via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_RETURNED, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 Digimon card with the [Xros Heart]/[Blue Flare] trait from your hand or trash under your Tamers.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT19-051 Place 1 [Xros Heart]/[Blue Flare] card from trash under 1 of your Tamers, then <Save>")
        effect4.set_effect_description("[On Deletion] Place 1 Digimon card with the [Xros Heart]/[Blue Flare] trait from your hand or trash under your Tamers.")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: blocker
        # Blocker
        effect5 = ICardEffect()
        effect5.set_effect_name("BT19-051 Blocker")
        effect5.set_effect_description("Blocker")
        effect5.is_inherited_effect = True
        effect5._is_blocker = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
