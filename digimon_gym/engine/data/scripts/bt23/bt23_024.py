from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_024(CardScript):
    """BT23-024"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-024 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-024 Effect")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Oujamon') or permanent.contains_card_name('Beautymon'))):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: evade
        # Evade
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-024 Evade")
        effect2.set_effect_description("Evade")
        effect2._is_evade = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [When Attacking] You may link 1 Digimon card with the [Appmon] trait from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-024 Link 1 digimon with [Appmon] trait from hand or this card's digivolution cards")
        effect3.set_effect_description("[When Digivolving] [When Attacking] You may link 1 Digimon card with the [Appmon] trait from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] You may link 1 Digimon card with the [Appmon] trait from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-024 Link 1 digimon with [Appmon] trait from hand or this card's digivolution cards")
        effect4.set_effect_description("[When Attacking] You may link 1 Digimon card with the [Appmon] trait from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect4.is_on_attack = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenLinked
        # [All Turns] [Once Per Turn] When this Digimon gets linked, by unsuspending it, other than their highest play cost Digimon, none of your opponent's Digimon can suspend until their turn ends.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT23-024 By unsuspending this digimon, all but their highest play cost digimon cant suspend")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When this Digimon gets linked, by unsuspending it, other than their highest play cost Digimon, none of your opponent's Digimon can suspend until their turn ends.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT23-024_WL")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
