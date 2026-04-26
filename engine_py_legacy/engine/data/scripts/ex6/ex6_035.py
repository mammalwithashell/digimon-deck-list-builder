from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_035(CardScript):
    """EX6-035 Cherubimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-035 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-035 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Cherubimon] for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_name = "Antylamon"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Cherubimon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: alliance
        # Alliance
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-035 Alliance")
        effect2.set_effect_description("Alliance")
        effect2.is_on_attack = True
        effect2._is_alliance = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 level 4 or lower green or yellow Digimon card from your hand without paying the cost. Then, 1 of your opponent's Digimon gets -4000 DP for each of your other Digimon until the end of their turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX6-035 Play 1 Digimon from hand, Then -4k DP for each digimon")
        effect3.set_effect_description("[On Play] You may play 1 level 4 or lower green or yellow Digimon card from your hand without paying the cost. Then, 1 of your opponent's Digimon gets -4000 DP for each of your other Digimon until the end of their turn.")
        effect3.set_hash_string("Play1Digimon_EX6_035")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not ('Green' in [col.name for col in getattr(c, 'card_colors', [])] or 'Yellow' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Then, 1 opponent Digimon gets -4000 DP per your other Digimon
            other_digimon = sum(1 for p in player.battle_area if p.is_digimon and p is not perm)
            if other_digimon > 0:
                dp_reduction = -4000 * other_digimon
                def on_dp_target(target_perm, dr=dp_reduction):
                    target_perm.change_dp(dr)
                game.effect_select_opponent_permanent(
                    player, on_dp_target,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt=f"Select 1 opponent Digimon to give {dp_reduction} DP.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 level 4 or lower green or yellow Digimon card from your hand without paying the cost. Then, 1 of your opponent's Digimon gets -4000 DP for each of your other Digimon until the end of their turn.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX6-035 Play 1 Digimon from hand, Then -4k DP for each digimon")
        effect4.set_effect_description("[When Digivolving] You may play 1 level 4 or lower green or yellow Digimon card from your hand without paying the cost. Then, 1 of your opponent's Digimon gets -4000 DP for each of your other Digimon until the end of their turn.")
        effect4.set_hash_string("Play1Digimon_EX6_035")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not ('Green' in [col.name for col in getattr(c, 'card_colors', [])] or 'Yellow' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Inherited: Ace Overflow -4
        effect5 = ICardEffect()
        effect5.set_effect_name("EX6-035 Ace Overflow -4")
        effect5.set_effect_description("Ace Overflow -4")
        effect5.is_inherited_effect = True
        effect5._is_ace_overflow = True
        effect5._overflow_amount = -4

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
