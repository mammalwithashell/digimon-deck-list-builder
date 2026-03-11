from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_101(CardScript):
    """BT23-101 Hudiemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-101 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 w/[CS] trait for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-101 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Erika Mishima] for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_name = "Erika Mishima"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Erika Mishima'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: alliance
        # Alliance
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-101 Alliance")
        effect2.set_effect_description("Alliance")
        effect2.is_on_attack = True
        effect2._is_alliance = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 play cost 5 or lower [CS] trait card from your hand without paying the cost. Then, to 1 of your opponent's Digimon, give -3000 DP for the turn for each of your [Hudie] trait Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT23-101 Play 1 5 cost or lower [CS] card from hand, then 1 digimon gains -3000 for each of your [Hudie] digimon")
        effect3.set_effect_description("[On Play] You may play 1 play cost 5 or lower [CS] trait card from your hand without paying the cost. Then, to 1 of your opponent's Digimon, give -3000 DP for the turn for each of your [Hudie] trait Digimon.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play 1 cost 5- CS card from hand, then -3000 DP per Hudie Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'play_cost', 99) > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
                return True

            def after_play():
                # Give -3000 DP per Hudie Digimon to 1 opponent Digimon
                hudie_count = sum(
                    1 for p in player.battle_area
                    if p.is_digimon and any('Hudie' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                )
                if hudie_count > 0:
                    dp_change = -3000 * hudie_count
                    def dp_filter(p):
                        return p.is_digimon and p.dp is not None
                    def on_dp_reduce(target_perm):
                        target_perm.change_dp(dp_change)
                    game.effect_select_opponent_permanent(
                        player, on_dp_reduce, filter_fn=dp_filter, is_optional=False,
                        prompt=f"Select 1 opponent Digimon to give {dp_change} DP.")

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            after_play()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 play cost 5 or lower [CS] trait card from your hand without paying the cost. Then, to 1 of your opponent's Digimon, give -3000 DP for the turn for each of your [Hudie] trait Digimon.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-101 Play 1 5 cost or lower [CS] card from hand, then 1 digimon gains -3000 for each of your [Hudie] digimon")
        effect4.set_effect_description("[When Digivolving] You may play 1 play cost 5 or lower [CS] trait card from your hand without paying the cost. Then, to 1 of your opponent's Digimon, give -3000 DP for the turn for each of your [Hudie] trait Digimon.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play 1 cost 5- CS card from hand, then -3000 DP per Hudie Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'play_cost', 99) > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
                return True

            def after_play():
                hudie_count = sum(
                    1 for p in player.battle_area
                    if p.is_digimon and any('Hudie' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                )
                if hudie_count > 0:
                    dp_change = -3000 * hudie_count
                    def dp_filter(p):
                        return p.is_digimon and p.dp is not None
                    def on_dp_reduce(target_perm):
                        target_perm.change_dp(dp_change)
                    game.effect_select_opponent_permanent(
                        player, on_dp_reduce, filter_fn=dp_filter, is_optional=False,
                        prompt=f"Select 1 opponent Digimon to give {dp_change} DP.")

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            after_play()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] By returning 1 of your [CS] trait Tamers to the hand, activate 1 of this Digimon's [On Play] effects.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("BT23-101 By bounce 1 [CS] tamer to hand, Activate 1 [On Play]")
        effect5.set_effect_description("[When Attacking] [Once Per Turn] By returning 1 of your [CS] trait Tamers to the hand, activate 1 of this Digimon's [On Play] effects.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT23_101_WA")
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
