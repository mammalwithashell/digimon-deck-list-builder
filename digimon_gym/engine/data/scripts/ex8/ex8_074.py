from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_074(CardScript):
    """EX8-074 MedievalGallantmon | Lv.6 | Green/Red | Cost:11 | DP:11000"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── BeforePayCost ─────────────────────────────────────────────────────
        # When this card would be played, by suspending 2 Digimon, reduce play cost by 4.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("EX8-074 Suspend 2 Digimon to get Play Cost -4")
        effect0.set_effect_description(
            "When this card would be played, by suspending 2 Digimon, "
            "reduce the play cost by 4."
        )
        effect0.set_hash_string("PlayCost-4_EX8_074")
        effect0.cost_reduction = 4

        def condition0(context: Dict[str, Any]) -> bool:
            # LEAK GUARD: only for THIS card being played
            if context.get('card_source') is not card:
                return False
            # C# CanActivateCondition: need 2+ unsuspended Digimon on ANY field
            # (IsPermanentExistsOnBattleAreaDigimon checks both fields)
            if card and card.owner:
                enemy = card.owner.enemy if hasattr(card.owner, 'enemy') else None
                all_digimon = [p for p in card.owner.battle_area
                               if p.is_digimon and not p.is_suspended]
                if enemy:
                    all_digimon += [p for p in enemy.battle_area
                                    if p.is_digimon and not p.is_suspended]
                return len(all_digimon) >= 2
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Cost: Player selects 2 own unsuspended Digimon to suspend."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            own_digimon = [p for p in player.battle_area
                           if p.is_digimon and not p.is_suspended]
            if len(own_digimon) < 2:
                return

            # Track how many have been suspended
            selected = []

            def on_first_selected(target_perm):
                target_perm.suspend()
                selected.append(target_perm)
                # Now select second
                def on_second_selected(target_perm2):
                    target_perm2.suspend()
                game.effect_select_own_permanent(
                    player, on_second_selected,
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended and p not in selected,
                    is_optional=False,
                    prompt="Select 2nd Digimon to suspend (cost for -4 play cost).",
                )

            game.effect_select_own_permanent(
                player, on_first_selected,
                filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                is_optional=False,
                prompt="Select 1st Digimon to suspend (cost for -4 play cost).",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Alliance ──────────────────────────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-074 Alliance")
        effect1.set_effect_description("Alliance")
        effect1.is_on_attack = True
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ── Vortex ────────────────────────────────────────────────────────────
        effect2 = ICardEffect()
        effect2.set_effect_name("EX8-074 Vortex")
        effect2.set_effect_description("Vortex")
        effect2._is_vortex = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # ── [When Digivolving] ────────────────────────────────────────────────
        # You may suspend 1 Digimon. Then, you may delete 1 of your opponent's
        # 8000 DP or lower Digimon. For each other suspended Digimon, add 3000 to
        # this DP deletion effect's maximum.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX8-074 Suspend then Delete")
        effect3.set_effect_description(
            "[When Digivolving] You may suspend 1 Digimon. Then, you may "
            "delete 1 of your opponent's 8000 DP or lower Digimon. For each "
            "other suspended Digimon, add 3000 to this DP deletion effect's maximum."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def _do_delete(player, game):
            """Step 2: You MAY delete 1 opponent Digimon with DP <= base + scaling.
            'Other suspended Digimon' = all suspended Digimon except this card's permanent."""
            enemy = player.enemy if player else None
            this_perm = card.permanent_of_this_card() if card else None
            all_suspended = sum(
                1 for p in (list(player.battle_area) + list(enemy.battle_area if enemy else []))
                if p.is_digimon and p.is_suspended and p is not this_perm
            )
            max_dp = 8000 + 3000 * all_suspended

            def delete_filter(p):
                return p.is_digimon and (p.dp or 0) <= max_dp

            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)

            opp_targets = [p for p in (enemy.battle_area if enemy else []) if delete_filter(p)]
            if opp_targets:
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=delete_filter,
                    is_optional=True,
                    prompt=f"You may delete 1 opponent Digimon with {max_dp} DP or less.")

        def _suspend_any_then_delete(player, game):
            """Step 1: You MAY suspend 1 Digimon (any on the field — own or opponent's).
            Then proceed to deletion step regardless of whether a suspension happened.
            C# ref: CanSelectSuspendPermanentCondition uses IsPermanentExistsOnBattleAreaDigimon
            which includes all Digimon on both fields."""
            enemy = player.enemy if player else None

            # Check if any Digimon exist on either field
            has_opp_digimon = any(p.is_digimon for p in (enemy.battle_area if enemy else []))
            has_own_digimon = any(p.is_digimon for p in player.battle_area)

            if not has_opp_digimon and not has_own_digimon:
                _do_delete(player, game)
                return

            # Card text says "1 Digimon" without restriction — any Digimon on the field.
            # Since the engine has no combined selector, offer opponent Digimon first
            # (more strategically useful), then own.
            suspended_happened = [False]

            def on_suspend_opp(target_perm):
                target_perm.suspend()
                suspended_happened[0] = True
                _do_delete(player, game)

            def on_suspend_own(target_perm):
                target_perm.suspend()
                suspended_happened[0] = True
                _do_delete(player, game)

            if has_opp_digimon:
                game.effect_select_opponent_permanent(
                    player, on_suspend_opp,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=True,
                    prompt="You may suspend 1 Digimon (opponent's).")

                if game.pending_selection:
                    _orig_decline = getattr(game.pending_selection, 'on_decline', None)
                    def on_decline_opp():
                        if _orig_decline:
                            _orig_decline()
                        # Declined opponent — offer own Digimon
                        if has_own_digimon:
                            game.effect_select_own_permanent(
                                player, on_suspend_own,
                                filter_fn=lambda p: p.is_digimon,
                                is_optional=True,
                                prompt="You may suspend 1 of your Digimon.")
                            if game.pending_selection:
                                _orig_decline2 = getattr(game.pending_selection, 'on_decline', None)
                                def on_decline_own():
                                    if _orig_decline2:
                                        _orig_decline2()
                                    _do_delete(player, game)
                                game.pending_selection.on_decline = on_decline_own
                            elif not suspended_happened[0]:
                                _do_delete(player, game)
                        else:
                            _do_delete(player, game)
                    game.pending_selection.on_decline = on_decline_opp
                elif not suspended_happened[0]:
                    # No opponent Digimon after filter or resolved synchronously
                    if has_own_digimon:
                        game.effect_select_own_permanent(
                            player, on_suspend_own,
                            filter_fn=lambda p: p.is_digimon,
                            is_optional=True,
                            prompt="You may suspend 1 of your Digimon.")
                        if game.pending_selection:
                            _orig_decline3 = getattr(game.pending_selection, 'on_decline', None)
                            def on_decline_own2():
                                if _orig_decline3:
                                    _orig_decline3()
                                _do_delete(player, game)
                            game.pending_selection.on_decline = on_decline_own2
                        elif not suspended_happened[0]:
                            _do_delete(player, game)
                    else:
                        _do_delete(player, game)
            elif has_own_digimon:
                game.effect_select_own_permanent(
                    player, on_suspend_own,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=True,
                    prompt="You may suspend 1 of your Digimon.")
                if game.pending_selection:
                    _orig_decline4 = getattr(game.pending_selection, 'on_decline', None)
                    def on_decline_own3():
                        if _orig_decline4:
                            _orig_decline4()
                        _do_delete(player, game)
                    game.pending_selection.on_decline = on_decline_own3
                elif not suspended_happened[0]:
                    _do_delete(player, game)
            else:
                _do_delete(player, game)

        def process3(ctx: Dict[str, Any]):
            """You may suspend 1 Digimon (any), then optionally delete opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _suspend_any_then_delete(player, game)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ── [All Turns] [Once Per Turn] ───────────────────────────────────────
        # When Digimon are played, you may activate 1 of this Digimon's
        # [When Digivolving] effects.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX8-074 Replay When Digivolving on play")
        effect4.set_effect_description(
            "[All Turns] [Once Per Turn] When Digimon are played, you may "
            "activate 1 of this Digimon's [When Digivolving] effects."
        )
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("PlayActivate_EX8_074")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be triggered by a Digimon being PLAYED (not digivolved)
            played_perm = context.get('played_permanent') or context.get('permanent')
            is_digivolve = context.get('is_digivolve', False)
            if is_digivolve:
                return False
            if played_perm is None:
                return False
            if not getattr(played_perm, 'is_digimon', False):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Re-run the [When Digivolving] effect (suspend + delete)."""
            process3(ctx)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
