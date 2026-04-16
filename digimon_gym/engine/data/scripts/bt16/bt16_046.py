from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_046(CardScript):
    """BT16-046 GranKuwagamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-046 Blast Digivolve")
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
        effect1.set_effect_name("BT16-046 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_name = "Dinobeemon"

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ── Shared helper: Suspend 2 opp Digimon/Tamers + cannot unsuspend + delete 1 suspended Tamer ──
        def _suspend_and_delete(ctx: Dict[str, Any]):
            """Suspend 2 opp Digimon/Tamers (CANNOT_UNSUSPEND via modifier),
            then delete 1 of their suspended Tamers.
            Selections are chained: 1st suspend -> 2nd suspend -> delete tamer."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            from ....interfaces.modifiers import ModifierType

            # Track targets chosen by this effect so selections don't repeat
            already_chosen: list = []

            def suspend_filter(p):
                return (p.is_digimon or p.is_tamer) and p not in already_chosen

            def _delete_suspended_tamer():
                """Final step: delete 1 of opponent's suspended Tamers."""
                def suspended_tamer_filter(p):
                    return p.is_tamer and p.is_suspended
                game.effect_select_opponent_permanent(
                    player, lambda tp: enemy.delete_permanent(tp),
                    filter_fn=suspended_tamer_filter, is_optional=False)

            def _suspend_second():
                """Second suspend selection, chained from the first."""
                opp_targets_2 = [p for p in enemy.battle_area if suspend_filter(p)]
                if not opp_targets_2:
                    _delete_suspended_tamer()
                    return

                def on_suspend_2(target_perm):
                    already_chosen.append(target_perm)
                    was_already_suspended = target_perm.is_suspended
                    target_perm.suspend()
                    if not was_already_suspended:
                        game.register_modifier(
                            target_perm, ModifierType.CANNOT_UNSUSPEND,
                            expiry='end_of_opponent_turn')
                    _delete_suspended_tamer()

                game.effect_select_opponent_permanent(
                    player, on_suspend_2, filter_fn=suspend_filter, is_optional=False)

            # First suspend selection
            opp_targets_1 = [p for p in enemy.battle_area
                             if p.is_digimon or p.is_tamer]
            if not opp_targets_1:
                _delete_suspended_tamer()
                return

            def on_suspend_1(target_perm):
                already_chosen.append(target_perm)
                was_already_suspended = target_perm.is_suspended
                target_perm.suspend()
                if not was_already_suspended:
                    game.register_modifier(
                        target_perm, ModifierType.CANNOT_UNSUSPEND,
                        expiry='end_of_opponent_turn')
                _suspend_second()

            game.effect_select_opponent_permanent(
                player, on_suspend_1,
                filter_fn=lambda p: p.is_digimon or p.is_tamer,
                is_optional=False)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 2 of your opponent's Digimon or Tamers. Cards suspended by this effect cant unsuspend during your opponent's next unsuspend phase. Then, delete 1 of your opponent's suspended Tamers.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT16-046 Suspend Digimon or tamers and activate effects")
        effect2.set_effect_description("[On Play] Suspend 2 of your opponent's Digimon or Tamers. Cards suspended by this effect cant unsuspend during your opponent's next unsuspend phase. Then, delete 1 of your opponent's suspended Tamers.")
        effect2.is_on_play = True
        effect2._is_cannot_unsuspend = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_suspend_and_delete)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 2 of your opponent's Digimon or Tamers. Cards suspended by this effect cant unsuspend during your opponent's next unsuspend phase. Then, delete 1 of your opponent's suspended Tamers.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT16-046 Suspend Digimon or tamers and activate effects")
        effect3.set_effect_description("[When Digivolving] Suspend 2 of your opponent's Digimon or Tamers. Cards suspended by this effect cant unsuspend during your opponent's next unsuspend phase. Then, delete 1 of your opponent's suspended Tamers.")
        effect3.is_when_digivolving = True
        effect3._is_cannot_unsuspend = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_suspend_and_delete)
        effects.append(effect3)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When this Digimon becomes suspended, 1 of your Digimon gains [Security A+1] for the turn.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnTappedAnyone)
        effect4.set_effect_name("BT16-046 Give 1 Digimon Security A+1.")
        effect4.set_effect_description("[Your Turn][Once Per Turn] When this Digimon becomes suspended, 1 of your Digimon gains [Security A+1] for the turn.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Sec+1_BT16-046")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be triggered by THIS Digimon becoming suspended
            ctx_perm = context.get('permanent')
            if ctx_perm is not card.permanent_of_this_card():
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Grant 1 of your Digimon Security A. +1 for the turn"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def digimon_filter(p):
                return p.is_digimon

            def on_sa_grant(target_perm):
                target_perm._temp_sa_modifier += 1

            game.effect_select_own_permanent(
                player, on_sa_grant, filter_fn=digimon_filter, is_optional=False,
                prompt="Select 1 Digimon to gain Security A. +1.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
