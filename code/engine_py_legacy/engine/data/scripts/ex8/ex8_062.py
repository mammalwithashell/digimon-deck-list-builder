from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_062(CardScript):
    """EX8-062 Piedmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-062 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 with [NSo] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "NSo"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('NSo' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-062 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Activate the effect below 4 times: • 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX8-062 Apply -2000 DP 4 times")
        effect2.set_effect_description("[On Play] Activate the effect below 4 times: • 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect2.is_on_play = True
        effect2.dp_modifier = -2000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-2000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Activate the effect below 4 times: • 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX8-062 Apply -2000 DP 4 times")
        effect3.set_effect_description("[When Digivolving] Activate the effect below 4 times: • 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect3.is_when_digivolving = True
        effect3.dp_modifier = -2000

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-2000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When other Digimon are deleted, you may play 1 level 4 or lower [NSo] trait Digimon card from your trash without paying the cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("EX8-062 Play 1 [NSo] Digimon from the trash")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When other Digimon are deleted, you may play 1 level 4 or lower [NSo] trait Digimon card from your trash without paying the cost.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("PlayDigimon_EX8_062")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
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
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
