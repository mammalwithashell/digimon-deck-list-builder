from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_007(CardScript):
    """EX5-007 Coronamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-007 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "Light Fang"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If you have a Tamer with the [Light Fang]/[Night Claw] trait, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX5-007 Memory +1")
        effect1.set_effect_description("[Start of Your Main Phase] If you have a Tamer with the [Light Fang]/[Night Claw] trait, gain 1 memory.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] [Once Per Turn] By placing the top card of this Digimon with the [Light Fang]/[Night Claw] trait as this Digimon's bottom digivolution card, gain 2 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("EX5-007 Place the top card of this Digimon at the bottom of digivolution cards to gain Memory +2 (Coronamon)")
        effect2.set_effect_description("[Main] [Once Per Turn] By placing the top card of this Digimon with the [Light Fang]/[Night Claw] trait as this Digimon's bottom digivolution card, gain 2 memory.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ReturnDigivolutionCards_EX50_007")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 1):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
