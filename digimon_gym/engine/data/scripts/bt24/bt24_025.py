from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_025(CardScript):
    """BT24-025 Shellmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.3 with [TS] trait for cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-025 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUnTappedAnyone
        # [Your Turn] When any of your OTHER BLUE Digimon with the [TS] trait unsuspend,
        # this Digimon may digivolve into [Venusmon] in the hand, ignoring level.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUnTappedAnyone)
        effect1.set_effect_name("BT24-025 Digivolve into [Venusmon] in the hand")
        effect1.set_effect_description("[Your Turn] When any of your other blue Digimon with the [TS] trait unsuspend, this Digimon may digivolve into [Venusmon] in the hand, ignoring level.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggering permanent must be one of our OTHER blue TS Digimon
            triggering_perm = context.get('permanent') if context else None
            my_perm = card.permanent_of_this_card() if card else None
            if triggering_perm is None:
                return False
            # Must not be this Digimon
            if triggering_perm == my_perm:
                return False
            # Must be on owner's battle area, be a Digimon
            if not getattr(triggering_perm, 'is_digimon', False):
                return False
            if getattr(triggering_perm, 'owner', None) != (card.owner if card else None):
                return False
            # Must be blue
            top = getattr(triggering_perm, 'top_card', None)
            if top is None:
                return False
            colors = getattr(top, 'card_colors', []) or []
            if 'Blue' not in colors and 'blue' not in colors:
                return False
            # Must have TS trait
            traits = getattr(top, 'card_traits', []) or []
            if not any('TS' in t for t in traits):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve into Venusmon ignoring level"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return any('Venusmon' in n for n in getattr(c, 'card_names', []))

            game.effect_digivolve_from_hand(
                player, perm, digi_filter,
                ignore_requirements=True,
                is_optional=True
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] 1 of your OTHER blue Digimon
        # with the [TS] trait may unsuspend.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT24-025 Unsuspend 1 other blue TS Digimon")
        effect2.set_effect_description("[End of Your Turn] [Once Per Turn] 1 of your other blue Digimon with the [TS] trait may unsuspend.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_025_EOYT")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Unsuspend 1 other blue TS Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            my_perm = card.permanent_of_this_card() if card else None

            def target_filter(p):
                # Must be a different Digimon of ours
                if p == my_perm:
                    return False
                if not getattr(p, 'is_digimon', False):
                    return False
                if getattr(p, 'owner', None) != player:
                    return False
                top = getattr(p, 'top_card', None)
                if top is None:
                    return False
                # Must have TS trait
                traits = getattr(top, 'card_traits', []) or []
                if not any('TS' in t for t in traits):
                    return False
                return True

            def on_unsuspend(target_perm):
                target_perm.unsuspend()

            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: jamming (inherited / ESS)
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-025 Jamming")
        effect3.set_effect_description("Jamming")
        effect3.is_inherited_effect = True
        effect3._is_jamming = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
