from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_043(CardScript):
    """BT24-043 Tapirmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-043 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.2 with [TS] trait for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the [Shaman] trait or with [Beast], [Animal], or [Sovereign], other than [Sea Animal], in any of its traits and 1 card with [TS] trait among them to the hand. Return the rest to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-043 Reveal 3 from deck. Add 2. Bottom deck the rest.")
        effect1.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the [Shaman] trait or with [Beast], [Animal], or [Sovereign], other than [Sea Animal], in any of its traits and 1 card with [TS] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reveal And Select"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter_0(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                # Matches [Shaman] exactly, or [Beast], [Animal], [Sovereign] as substrings, excluding [Sea Animal]
                has_shaman = any('Shaman' in _t for _t in traits)
                has_beast = any('Beast' in _t for _t in traits)
                has_animal = any('Animal' in _t for _t in traits)
                has_sovereign = any('Sovereign' in _t for _t in traits)
                has_sea_animal = any('Sea Animal' in _t for _t in traits)
                # [Sea Animal] is excluded from Animal matches
                if has_animal and has_sea_animal:
                    # Only counts if there's a non-Sea Animal "Animal" trait
                    has_animal = any('Animal' in _t and 'Sea Animal' not in _t for _t in traits)
                return has_shaman or has_beast or has_animal or has_sovereign

            def reveal_filter_1(c):
                if not (any('TS' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True

            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] Suspend 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT24-043 Suspend 1 opponent's Digimon")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] Suspend 1 of your opponent's Digimon.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_043_Inherited")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
