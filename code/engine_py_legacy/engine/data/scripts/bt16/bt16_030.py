from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_030(CardScript):
    """BT16-030 Salamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-030 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Nyaromon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Main Phase] If it's your turn, 1 of your Digimon may digivolve into a level 4 Digimon card with the [Holy Beast] or [Free] trait from your trash with the digivolution cost reduced by 1.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT16-030 1 of your Digimon may Digivolve")
        effect1.set_effect_description("[Start of Main Phase] If it's your turn, 1 of your Digimon may digivolve into a level 4 Digimon card with the [Holy Beast] or [Free] trait from your trash with the digivolution cost reduced by 1.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Digivolve_BT16_030")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                _traits = getattr(c, 'card_traits', []) or []
                _attrs = getattr(c.c_entity_base, 'attribute_eng', []) or []
                if not (any('Holy Beast' in _t or 'HolyBeast' in _t for _t in _traits) or 'Free' in _attrs):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If it's your turn, 1 of your Digimon may digivolve into a level 4 Digimon card with the [Holy Beast] or [Free] trait from your trash with the digivolution cost reduced by 1.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT16-030 1 of your Digimon may Digivolve")
        effect2.set_effect_description("[On Play] If it's your turn, 1 of your Digimon may digivolve into a level 4 Digimon card with the [Holy Beast] or [Free] trait from your trash with the digivolution cost reduced by 1.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Digivolve_BT16_030")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                _traits = getattr(c, 'card_traits', []) or []
                _attrs = getattr(c.c_entity_base, 'attribute_eng', []) or []
                if not (any('Holy Beast' in _t or 'HolyBeast' in _t for _t in _traits) or 'Free' in _attrs):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
