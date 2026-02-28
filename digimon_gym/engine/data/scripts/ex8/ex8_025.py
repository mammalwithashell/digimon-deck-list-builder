from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_025(CardScript):
    """EX8-025 Whamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-025 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [DS] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "DS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('DS' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-025 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 Digimon card with the [DS] trait from your trash as this Digimon's bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX8-025 Place 1 Digimon with [DS] trait from trash as this Digimon's bottom digivolution card")
        effect2.set_effect_description("[On Play] You may place 1 Digimon card with the [DS] trait from your trash as this Digimon's bottom digivolution card.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 Digimon card with the [DS] trait from your trash as this Digimon's bottom digivolution card.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX8-025 Place 1 Digimon with [DS] trait from trash as this Digimon's bottom digivolution card")
        effect3.set_effect_description("[When Digivolving] You may place 1 Digimon card with the [DS] trait from your trash as this Digimon's bottom digivolution card.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] [Once Per Turn] You may play 1 [DS] trait Digimon card with a play cost of 5 or less from this Digimon's digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX8-025 Play 1 digivolution card")
        effect4.set_effect_description("[End of Attack] [Once Per Turn] You may play 1 [DS] trait Digimon card with a play cost of 5 or less from this Digimon's digivolution cards without paying the cost.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EndofAttack_EX8_025")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 5:
                    return False
                if not (any('DS' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.None
        # Target Lock
        effect5 = ICardEffect()
        effect5.set_effect_name("EX8-025 This Digimon's attack target can't be switched.")
        effect5.set_effect_description("Target Lock")
        effect5.is_inherited_effect = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Target Lock"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Target lock — this Digimon's attack target can't be switched
            pass  # Handled by engine attack target resolution

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
