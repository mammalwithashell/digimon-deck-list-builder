from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_034(CardScript):
    """BT24-034 Aegiomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-034 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 from [Elecmon] with [TS] trait for cost 0
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Elecmon"
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Elecmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: barrier
        # Barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-034 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        def play_ts_tamer_after_security_to_hand(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.security_cards):
                return

            player.hand_cards.append(player.security_cards.pop(0))
            existing_tamer_names = {
                p.top_card.card_names[0]
                for p in player.battle_area
                if p.is_tamer and p.top_card and p.top_card.card_names
            }

            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                if 'TS' not in (getattr(c, 'card_traits', []) or []):
                    return False
                if any(name in existing_tamer_names for name in getattr(c, 'card_names', [])):
                    return False
                return True

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        # Timing: EffectTiming.OnMove
        # Add top security to hand, then play a [TS] Tamer
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnMove)
        effect2.set_effect_name("BT24-034 Add top security to hand, then play a [TS] Tamer")
        effect2.set_effect_description("By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost. This effect can't play cards with the same name as any of your Tamers.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        effect2.set_on_process_callback(play_ts_tamer_after_security_to_hand)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Add top security to hand, then play a [TS] Tamer
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-034 Add top security to hand, then play a [TS] Tamer")
        effect3.set_effect_description("By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost. This effect can't play cards with the same name as any of your Tamers.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        effect3.set_on_process_callback(play_ts_tamer_after_security_to_hand)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Add top security to hand, then play a [TS] Tamer
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-034 Add top security to hand, then play a [TS] Tamer")
        effect4.set_effect_description("By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost. This effect can't play cards with the same name as any of your Tamers.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        effect4.set_on_process_callback(play_ts_tamer_after_security_to_hand)
        effects.append(effect4)

        # Factory effect: barrier
        # Barrier
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-034 Barrier")
        effect5.set_effect_description("Barrier")
        effect5.is_inherited_effect = True
        effect5._is_barrier = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
