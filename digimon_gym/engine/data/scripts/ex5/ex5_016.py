from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_016(CardScript):
    """EX5-016 Lunamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.2 with [Night Claw] or [Light Fang] trait for cost 0
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-016 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "Night Claw"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Start of Your Main Phase] By returning 1 of your Digimon to hand, gain 2 memory.
        # "By returning" = cost that must happen BEFORE the memory gain
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX5-016 Return your 1 Digimon to hand to gain Memory +2")
        effect1.set_effect_description("[Start of Your Main Phase] By returning 1 of your Digimon to the hand, gain 2 memory.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """By returning 1 of your Digimon to hand (cost), gain 2 memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def own_digimon_filter(p):
                return p.is_digimon

            has_own = any(own_digimon_filter(p) for p in player.battle_area)
            if not has_own:
                return

            def on_bounce_cost(target_perm):
                # Cost: bounce own Digimon to hand
                player.bounce_permanent_to_hand(target_perm)
                # Effect: gain 2 memory
                player.add_memory(2)

            game.effect_select_own_permanent(
                player, on_bounce_cost, filter_fn=own_digimon_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: [Main] [Once Per Turn] By placing the top card of this Digimon
        # with the [Night Claw] or [Light Fang] trait as this Digimon's bottom
        # digivolution card, gain 2 memory.
        # "By placing" = cost that must happen BEFORE the memory gain
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("EX5-016 Place the top card of this Digimon at the bottom of digivolution cards to gain Memory +2 (Lunamon)")
        effect2.set_effect_description("[Main] [Once Per Turn] By placing the top card of this Digimon with the [Night Claw] or [Light Fang] trait as this Digimon's bottom digivolution card, gain 2 memory.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ReturnDigivolutionCards_EX50_016")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect2.effect_source_permanent if hasattr(effect2, 'effect_source_permanent') else None
            if not permanent:
                permanent = card.permanent_of_this_card()
            if not permanent:
                return False
            # Need at least 1 digivolution card (so we have a top card to move)
            if len(permanent.digivolution_cards) < 1:
                return False
            # Top card must have Night Claw or Light Fang trait
            top = permanent.top_card
            if not top:
                return False
            traits = getattr(top, 'card_traits', []) or []
            has_required_trait = any(
                t in ('Night Claw', 'NightClaw', 'Light Fang', 'LightFung', 'LightFang')
                for t in traits
            )
            return has_required_trait
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """By placing top card as bottom digi-card (cost), gain 2 memory."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            # Cost: place top card as bottom digivolution card
            if len(perm.digivolution_cards) >= 1:
                top_card = perm.top_card
                if top_card:
                    perm.add_card_source_bottom(top_card)
                    # Effect: gain 2 memory
                    player.add_memory(2)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
