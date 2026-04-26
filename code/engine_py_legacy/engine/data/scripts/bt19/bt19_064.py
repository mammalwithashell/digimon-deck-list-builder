from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_064(CardScript):
    """BT19-064 Justimon: Blitz Arm | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-064 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Justimon: Accel Arm]/[Justimon: Critical Arm] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Justimon: Accel Arm"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Justimon: Accel Arm') or permanent.contains_card_name('Justimon: Critical Arm'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-064 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] This Digimon gains Blocker for the turn.
        # Then, 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT19-064 Gain Blocker, then -5000 DP to 1 opp Digimon")
        effect2.set_effect_description("[On Play] This Digimon gains <Blocker> for the turn. Then, 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect2.is_on_play = True
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Blocker for the turn, then -5000 DP to 1 opp Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (perm and game):
                return

            # Gain Blocker for the turn
            perm.grant_keyword('_is_blocker', duration=game.turn_count)

            # Then, 1 of opponent's Digimon gets -5000 DP for the turn
            enemy = player.enemy if player else None
            if not enemy:
                return

            def dp_filter(p):
                return p.is_digimon and p.dp is not None

            def on_dp_target(target_perm):
                if target_perm:
                    target_perm.change_dp(-5000)

            if any(dp_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_dp_target, filter_fn=dp_filter, is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to get -5000 DP.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] This Digimon gains Blocker for the turn.
        # Then, 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT19-064 Gain Blocker, then -5000 DP to 1 opp Digimon")
        effect3.set_effect_description("[When Digivolving] This Digimon gains <Blocker> for the turn. Then, 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect3.is_when_digivolving = True
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain Blocker for the turn, then -5000 DP to 1 opp Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (perm and game):
                return

            # Gain Blocker for the turn
            perm.grant_keyword('_is_blocker', duration=game.turn_count)

            # Then, 1 of opponent's Digimon gets -5000 DP for the turn
            enemy = player.enemy if player else None
            if not enemy:
                return

            def dp_filter(p):
                return p.is_digimon and p.dp is not None

            def on_dp_target(target_perm):
                if target_perm:
                    target_perm.change_dp(-5000)

            if any(dp_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_dp_target, filter_fn=dp_filter, is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to get -5000 DP.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
