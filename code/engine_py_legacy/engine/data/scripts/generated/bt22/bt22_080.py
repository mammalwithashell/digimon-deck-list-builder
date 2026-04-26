from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_080(CardScript):
    """BT22-080 Eater (Human Form)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-080 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Eater (Species Form)] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Eater (Species Form)"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Eater (Species Form)'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 [Eater (Species Form)] from this Digimon's digivolution cards as the bottom digivolution card of your [Mother Eater] in the breeding area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-080 Move 1 [Eater (Species Form)] from sources under [Mother Eater] in breeding")
        effect1.set_effect_description("[When Digivolving] You may place 1 [Eater (Species Form)] from this Digimon's digivolution cards as the bottom digivolution card of your [Mother Eater] in the breeding area.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother Eater'))):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnSecurityCheck
        # [Your Turn] [Once Per Turn] When this Digimon checks your opponent's security stack, you may play 1 Tamer card with the [CS] trait from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-080 Play 1 [CS] Tamer")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon checks your opponent's security stack, you may play 1 Tamer card with the [CS] trait from your hand without paying the cost.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_080_Play_Tamer")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on attack — validated by engine timing
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
                if not (any('Eater' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Cost -1
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-080 Play Cost -1")
        effect3.set_effect_description("Cost -1")
        effect3.cost_reduction = 1

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
