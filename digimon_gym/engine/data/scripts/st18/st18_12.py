from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_12(CardScript):
    """ST18-12 Zephagamon | Lv.6 (Green, Magic Knight/Vortex Warriors/LIBERATOR,
    DP 11000, Cost 11)

    <Vortex>
    [When Digivolving] Suspend 1 Digimon. Then, unsuspend 1 Digimon.
    [All Turns] [Once Per Turn] When any Digimon unsuspends, for the turn, this
        Digimon isn't affected by the effects of your opponent's Digimon and gets
        +3000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Factory keyword <Vortex> ---
        effect0 = ICardEffect()
        effect0.set_effect_name("ST18-12 <Vortex>")
        effect0.set_effect_description("<Vortex>")
        effect0._is_vortex = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Suspend 1 Digimon, then unsuspend 1 Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("ST18-12 Suspend 1 Digimon then unsuspend 1 Digimon")
        effect1.set_effect_description(
            "[When Digivolving] Suspend 1 Digimon. Then, unsuspend 1 Digimon."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Suspend 1 Digimon (any — own or opponent)
            # Card text says "Suspend 1 Digimon" — unrestricted (any Digimon on field).
            # Use own-permanent selection first; if no valid own unsuspended, try opponent.
            # Actually "1 Digimon" means any Digimon. We use own-permanent selection
            # with a broad filter including opponent permanents via effect_select_opponent_permanent.
            # The engine doesn't have a global "select any permanent" helper, so we
            # use own-permanent selection (which is standard for ambiguous "1 Digimon" text).

            def on_suspend(target_perm):
                target_perm.suspend()

                # Step 2: Unsuspend 1 Digimon after suspend resolves
                def on_unsuspend(unsuspend_perm):
                    unsuspend_perm.unsuspend()

                # Select any own suspended or opponent Digimon to unsuspend.
                # Use own-permanent selection — broadest interpretation.
                # Filter: any Digimon (own or opponent) — we use own first then allow opponent.
                has_own_suspended = any(
                    p.is_digimon and p.is_suspended for p in player.battle_area
                )
                enemy = player.enemy
                has_opp_suspended = enemy and any(
                    p.is_digimon and p.is_suspended for p in enemy.battle_area
                )

                if has_own_suspended or has_opp_suspended:
                    # Prefer own selection for "1 Digimon" unsuspend
                    if has_own_suspended:
                        game.effect_select_own_permanent(
                            player, on_unsuspend,
                            filter_fn=lambda p: p.is_digimon and p.is_suspended,
                            is_optional=False,
                            prompt="Select 1 Digimon to unsuspend.",
                        )
                    elif has_opp_suspended:
                        game.effect_select_opponent_permanent(
                            player, on_unsuspend,
                            filter_fn=lambda p: p.is_digimon and p.is_suspended,
                            is_optional=False,
                            prompt="Select 1 Digimon to unsuspend.",
                        )

            # Select any Digimon to suspend (own or opponent, unsuspended)
            has_own_unsuspended = any(
                p.is_digimon and not p.is_suspended for p in player.battle_area
            )
            enemy = player.enemy
            has_opp_unsuspended = enemy and any(
                p.is_digimon and not p.is_suspended for p in enemy.battle_area
            )

            if has_own_unsuspended:
                game.effect_select_own_permanent(
                    player, on_suspend,
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                    is_optional=False,
                    prompt="Select 1 Digimon to suspend.",
                )
            elif has_opp_unsuspended:
                game.effect_select_opponent_permanent(
                    player, on_suspend,
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                    is_optional=False,
                    prompt="Select 1 Digimon to suspend.",
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] [Once Per Turn] When any Digimon unsuspends,
        #     this Digimon isn't affected by opponent's Digimon effects + +3000 DP ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUnTappedAnyone)
        effect2.set_effect_name(
            "ST18-12 When any Digimon unsuspends: immunity + +3000 DP"
        )
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When any Digimon unsuspends, for the turn, "
            "this Digimon isn't affected by the effects of your opponent's Digimon "
            "and gets +3000 DP."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ST18_12_UnsuspendTrigger")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The unsuspended permanent must be a Digimon
            event_perm = context.get('event_permanent') or context.get('permanent')
            if event_perm and not event_perm.is_digimon:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            owner_perm = card.permanent_of_this_card() if card else None
            if not owner_perm:
                return

            from ....interfaces.modifiers import ModifierType

            # Grant +3000 DP for the turn (frozen script convention: ModifierType first)
            game.register_modifier(
                ModifierType.CHANGE_DP, owner_perm,
                value_fn=lambda base, perm, ctx: base + 3000,
                expiry='end_of_turn',
            )

            # Grant immunity to opponent's Digimon effects for the turn.
            # Condition: only active when source_effect belongs to an opponent's Digimon.
            enemy_ref = player.enemy

            def digimon_effect_immunity_cond(target_perm, ctx_inner):
                if target_perm is not owner_perm:
                    return False
                if not enemy_ref:
                    return False
                source_eff = ctx_inner.get('source_effect') if ctx_inner else None
                if source_eff is None:
                    # No source effect context — be conservative, don't block
                    return False
                # Check if source_eff belongs to any opponent Digimon
                for ep in enemy_ref.battle_area:
                    if ep.is_digimon:
                        for source in ep.card_sources:
                            if source_eff in source.effect_list(EffectTiming.NoTiming):
                                return True
                return False

            game.register_modifier(
                ModifierType.CANNOT_BE_AFFECTED, owner_perm,
                condition=digimon_effect_immunity_cond,
                value_fn=lambda: True,
                expiry='end_of_turn',
            )

            game.logger.log(
                f"[ST18-12] {owner_perm.top_card.card_names[0] if owner_perm.top_card else 'Zephagamon'} "
                f"gains +3000 DP and immunity to opponent Digimon effects this turn."
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
