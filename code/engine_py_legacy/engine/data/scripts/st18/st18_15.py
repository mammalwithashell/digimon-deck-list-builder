from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_15(CardScript):
    """ST18-15 Anemoi Embrace | Option"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Suspend 1 Digimon. If this effect suspended your Digimon,
        # return 1 of your opponent's suspended Digimon to the bottom of the deck.
        # Then, unsuspend 1 Digimon.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST18-15 Suspend, bounce if own, unsuspend")
        effect0.set_effect_description(
            "[Main] Suspend 1 Digimon. If this effect suspended your Digimon, "
            "return 1 of your opponent's suspended Digimon to the bottom of "
            "the deck. Then, unsuspend 1 Digimon."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            suspended_own = [False]

            # Step 1: Suspend 1 Digimon (any — opponent or own)
            # Try opponent first, then own
            def opp_digimon_filter(p):
                return p.is_digimon and not p.is_suspended

            def own_digimon_filter(p):
                return p.is_digimon and not p.is_suspended

            def on_suspend_opp(target_perm):
                target_perm.suspend()
                suspended_own[0] = False

            def on_suspend_own(target_perm):
                target_perm.suspend()
                suspended_own[0] = True
                # If suspended own Digimon, return 1 opponent's suspended Digimon to deck bottom
                def bounce_filter(p):
                    return p.is_digimon and p.is_suspended

                def on_bounce(target_perm2):
                    enemy.return_permanent_to_deck_bottom(target_perm2)

                has_suspended_opp = any(bounce_filter(p) for p in enemy.battle_area)
                if has_suspended_opp:
                    game.effect_select_opponent_permanent(
                        player, on_bounce,
                        filter_fn=bounce_filter,
                        is_optional=False,
                        prompt="Return 1 of opponent's suspended Digimon to the bottom of the deck."
                    )

            # Offer opponent Digimon first
            has_opp = any(opp_digimon_filter(p) for p in enemy.battle_area)
            has_own = any(own_digimon_filter(p) for p in player.battle_area)

            if has_opp and has_own:
                # Let player choose: suspend opponent or own
                def on_choice(choice: int):
                    if choice == 0:
                        # Suspend opponent's Digimon
                        game.effect_select_opponent_permanent(
                            player, on_suspend_opp,
                            filter_fn=opp_digimon_filter,
                            is_optional=False,
                            prompt="Suspend 1 of your opponent's Digimon."
                        )
                    else:
                        # Suspend own Digimon
                        game.effect_select_own_permanent(
                            player, on_suspend_own,
                            filter_fn=own_digimon_filter,
                            is_optional=False,
                            prompt="Suspend 1 of your Digimon."
                        )

                game.effect_choose_branch(
                    player, 2, on_choice,
                    prompt="Choose: (0) Suspend opponent's Digimon, (1) Suspend your Digimon (enables bounce)."
                )
            elif has_opp:
                game.effect_select_opponent_permanent(
                    player, on_suspend_opp,
                    filter_fn=opp_digimon_filter,
                    is_optional=False,
                    prompt="Suspend 1 of your opponent's Digimon."
                )
            elif has_own:
                game.effect_select_own_permanent(
                    player, on_suspend_own,
                    filter_fn=own_digimon_filter,
                    is_optional=False,
                    prompt="Suspend 1 of your Digimon."
                )

            # Step 3: Unsuspend 1 Digimon (any — opponent or own)
            def any_suspended_opp(p):
                return p.is_digimon and p.is_suspended

            def any_suspended_own(p):
                return p.is_digimon and p.is_suspended

            def on_unsuspend_opp(target_perm):
                target_perm.unsuspend()

            def on_unsuspend_own(target_perm):
                target_perm.unsuspend()

            has_susp_opp = any(any_suspended_opp(p) for p in enemy.battle_area)
            has_susp_own = any(any_suspended_own(p) for p in player.battle_area)

            if has_susp_opp and has_susp_own:
                def on_unsuspend_choice(choice: int):
                    if choice == 0:
                        game.effect_select_opponent_permanent(
                            player, on_unsuspend_opp,
                            filter_fn=any_suspended_opp,
                            is_optional=False,
                            prompt="Unsuspend 1 Digimon (opponent's)."
                        )
                    else:
                        game.effect_select_own_permanent(
                            player, on_unsuspend_own,
                            filter_fn=any_suspended_own,
                            is_optional=False,
                            prompt="Unsuspend 1 Digimon (yours)."
                        )

                game.effect_choose_branch(
                    player, 2, on_unsuspend_choice,
                    prompt="Choose: (0) Unsuspend opponent's Digimon, (1) Unsuspend your Digimon."
                )
            elif has_susp_own:
                game.effect_select_own_permanent(
                    player, on_unsuspend_own,
                    filter_fn=any_suspended_own,
                    is_optional=False,
                    prompt="Unsuspend 1 of your Digimon."
                )
            elif has_susp_opp:
                game.effect_select_opponent_permanent(
                    player, on_unsuspend_opp,
                    filter_fn=any_suspended_opp,
                    is_optional=False,
                    prompt="Unsuspend 1 of opponent's Digimon."
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security Effect [Security] Return 1 of your opponent's suspended
        # Digimon to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-15 Security: Bounce suspended")
        effect1.set_effect_description(
            "[Security] Return 1 of your opponent's suspended Digimon to the "
            "bottom of the deck."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def bounce_filter(p):
                return p.is_digimon and p.is_suspended

            def on_bounce(target_perm):
                enemy.return_permanent_to_deck_bottom(target_perm)

            has_suspended = any(bounce_filter(p) for p in enemy.battle_area)
            if has_suspended:
                game.effect_select_opponent_permanent(
                    player, on_bounce,
                    filter_fn=bounce_filter,
                    is_optional=False,
                    prompt="Return 1 of opponent's suspended Digimon to the bottom of the deck."
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
