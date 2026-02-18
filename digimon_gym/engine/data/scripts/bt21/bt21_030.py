from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_030(CardScript):
    """BT21-030 Shoutmon X7: Superior Mode | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-030 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Hero] trait for cost 5
        effect0._alt_digi_cost = 5
        effect0._alt_digi_trait = "Hero"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Hero' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-030 Effect")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.BeforePayCost
        # When you would play this card from your hand, by placing 1 of your [Shoutmon] as a digivolution card under this Digimon, reduce its play cost by 1 and place the cards in your trash as digivolution cards for a DigiXros.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-030 Play Cost -1 and select trash cards for a DigiXros")
        effect2.set_effect_description("When you would play this card from your hand, by placing 1 of your [Shoutmon] as a digivolution card under this Digimon, reduce its play cost by 1 and place the cards in your trash as digivolution cards for a DigiXros.")
        effect2.is_optional = True
        effect2.set_hash_string("PlayCost-1_BT12_112")
        effect2.cost_reduction = 1

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 10 stacked cards of 1 of your opponent's Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-030 Trash top 10 stack cards")
        effect3.set_effect_description("[On Play] Trash the top 10 stacked cards of 1 of your opponent's Digimon.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash the top 10 stacked cards of 1 of your opponent's Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-030 Trash top 10 stack cards")
        effect4.set_effect_description("[When Digivolving] Trash the top 10 stacked cards of 1 of your opponent's Digimon.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] You may return 1 of your opponent's Digimon with no digivolution cards to the bottom of the deck.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT21-030 Bottom deck digimon with no digivolution cards")
        effect5.set_effect_description("[When Attacking] [Once Per Turn] You may return 1 of your opponent's Digimon with no digivolution cards to the bottom of the deck.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BottomDeck_BT21-030")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
