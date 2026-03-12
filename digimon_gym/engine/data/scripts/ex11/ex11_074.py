from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_074(CardScript):
    """EX11-074 Vortexdramon | Lv.7, Green, Bird Dragon/Vortex Warriors/LIBERATOR, DP 14000, Cost 14

    <Piercing> <Vortex> <Blocker>
    [When Digivolving][When Attacking] You may suspend 1 Digimon. If this effect suspended your
        Digimon, until your opponent's turn ends, their Digimon's effects don't affect this Digimon
        and it gets +6000 DP.
    [All Turns][Once Per Turn] When any Digimon suspend, this Digimon may unsuspend. Then, this
        Digimon may battle 1 of your opponent's Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alternate Digivolve from [Shoto Kazama] for cost 6 ---
        effect_alt = ICardEffect()
        effect_alt.set_effect_name("EX11-074 Alternate digivolution requirement")
        effect_alt.set_effect_description("Alternate digivolution requirement")
        effect_alt._alt_digi_cost = 6
        effect_alt._alt_digi_name = "Shoto Kazama"

        def cond_alt(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (
                permanent.contains_card_name('Shoto Kazama')
                or permanent.contains_card_name('GrandGalemon')
            )):
                return False
            return True
        effect_alt.set_can_use_condition(cond_alt)
        effects.append(effect_alt)

        # --- Keyword: <Piercing> ---
        effect_piercing = ICardEffect()
        effect_piercing.set_effect_name("EX11-074 Piercing")
        effect_piercing.set_effect_description("Piercing")
        effect_piercing._is_piercing = True

        def cond_piercing(context: Dict[str, Any]) -> bool:
            return True
        effect_piercing.set_can_use_condition(cond_piercing)
        effects.append(effect_piercing)

        # --- Keyword: <Vortex> ---
        effect_vortex = ICardEffect()
        effect_vortex.set_effect_name("EX11-074 Vortex")
        effect_vortex.set_effect_description("Vortex")
        effect_vortex._is_vortex = True

        def cond_vortex(context: Dict[str, Any]) -> bool:
            return True
        effect_vortex.set_can_use_condition(cond_vortex)
        effects.append(effect_vortex)

        # --- Keyword: <Blocker> ---
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("EX11-074 Blocker")
        effect_blocker.set_effect_description("Blocker")
        effect_blocker._is_blocker = True

        def cond_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(cond_blocker)
        effects.append(effect_blocker)

        # --- Helper: make the suspend-and-maybe-grant-immunity effect ---
        # Used for both [When Digivolving] and [When Attacking].
        def make_suspend_immunity_effect(timing_label: str, is_when_digivolving: bool):
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone if is_when_digivolving else EffectTiming.OnUseAttack)
            eff.set_effect_name(
                f"EX11-074 [{timing_label}] May suspend 1 Digimon; if own, gain immunity +6k DP"
            )
            eff.set_effect_description(
                f"[{timing_label}] You may suspend 1 Digimon. If this effect suspended your "
                "Digimon, until your opponent's turn ends, their Digimon's effects don't affect "
                "this Digimon and it gets +6000 DP."
            )
            if is_when_digivolving:
                eff.is_when_digivolving = True
            else:
                eff.is_on_attack = True
            eff.is_optional = True

            def cond(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True
            eff.set_can_use_condition(cond)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                perm = card.permanent_of_this_card() if card else None
                if not (player and game and perm):
                    return

                from digimon_gym.engine.interfaces.modifiers import ModifierType

                suspended_own = [False]  # mutable flag

                def grant_immunity():
                    if suspended_own[0]:
                        game.register_modifier(
                            ModifierType.CANNOT_BE_AFFECTED,
                            perm,
                            value_fn=lambda: True,
                            expiry='end_of_opponent_turn',
                        )
                        game.register_modifier(
                            ModifierType.CHANGE_DP,
                            perm,
                            value_fn=lambda: 6000,
                            expiry='end_of_opponent_turn',
                        )

                def on_own_suspended(target_perm):
                    suspended_own[0] = True
                    target_perm.suspend()
                    grant_immunity()

                # "You may suspend 1 Digimon" — can target own or opponent.
                # "If this effect suspended YOUR Digimon" — immunity only if own.
                # Prefer own (to enable immunity path). Offer opponent if no own available.
                own_suspendable = [
                    p for p in player.battle_area
                    if p.is_digimon and not p.is_suspended
                ]
                opp_suspendable = (
                    [p for p in player.enemy.battle_area if p.is_digimon and not p.is_suspended]
                    if player.enemy else []
                )

                if own_suspendable:
                    game.effect_select_own_permanent(
                        player,
                        on_own_suspended,
                        filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                        is_optional=True,
                        prompt="You may suspend 1 of your Digimon (grants immunity if suspended).",
                    )
                elif opp_suspendable:
                    game.effect_select_opponent_permanent(
                        player,
                        lambda p: p.suspend(),
                        filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                        is_optional=True,
                        prompt="You may suspend 1 of your opponent's Digimon.",
                    )

            eff.set_on_process_callback(process)
            return eff

        effects.append(make_suspend_immunity_effect("When Digivolving", is_when_digivolving=True))
        effects.append(make_suspend_immunity_effect("When Attacking", is_when_digivolving=False))

        # --- [All Turns][Once Per Turn] When any Digimon suspend, this Digimon may unsuspend.
        #     Then, this Digimon may battle 1 of your opponent's Digimon. ---
        effect_tap = ICardEffect()
        effect_tap.set_timing(EffectTiming.OnTappedAnyone)
        effect_tap.set_effect_name(
            "EX11-074 [All Turns][OPT] When any Digimon suspends, may unsuspend, then may battle opponent's Digimon"
        )
        effect_tap.set_effect_description(
            "[All Turns][Once Per Turn] When any Digimon suspend, this Digimon may unsuspend. "
            "Then, this Digimon may battle 1 of your opponent's Digimon."
        )
        effect_tap.is_optional = True
        effect_tap.set_max_count_per_turn(1)
        effect_tap.set_hash_string("EX11_074_AT_Battle")

        def cond_tap(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_tap.set_can_use_condition(cond_tap)

        def process_tap(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            this_perm = card.permanent_of_this_card() if card else None
            if not this_perm:
                return

            # Step 1: This Digimon may unsuspend
            if this_perm.is_suspended:
                this_perm.unsuspend()

            # Step 2: This Digimon may battle 1 of your opponent's Digimon.
            # Use resolve_attack with without_suspend=True so it doesn't re-suspend.
            opp_digimon = [
                p for p in player.enemy.battle_area if p.is_digimon
            ] if player.enemy else []

            if opp_digimon:
                def on_battle_target(target_perm):
                    game.resolve_attack(
                        this_perm, target_perm,
                        without_suspend=True,
                    )

                game.effect_select_opponent_permanent(
                    player, on_battle_target,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=True,
                    prompt="This Digimon may battle 1 of your opponent's Digimon.",
                )

        effect_tap.set_on_process_callback(process_tap)
        effects.append(effect_tap)

        return effects
