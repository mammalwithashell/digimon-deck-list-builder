from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class AD1_024(CardScript):
    """AD1-024 Imperialdramon: Fighter Mode | Lv.6 Blue/Green | DP 13000 | Cost 13

    <Security A. +1>
    <Blocker>
    [When Digivolving] [When Attacking] [Once Per Turn] Return 1 of your
        opponent's lowest DP Digimon to the bottom of the deck.
    [All Turns] [Once Per Turn] When Digimon are played or digivolve, you
        may suspend 1 of your opponent's Digimon and unsuspend this Digimon.
        Then, if played or digivolved by effects, you may return 1 of your
        opponent's suspended Digimon to the bottom of the deck.

    Alt-Digi: From Lv.5 with [Hero] trait for cost 5.
              From [Imperialdramon: Dragon Mode] for cost 1.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-Digi 1: Lv.5 with [Hero] trait, cost 5 ───────────────
        alt_hero = ICardEffect()
        alt_hero.set_effect_name("AD1-024 Alt-digi: Lv.5 Hero trait, cost 5")
        alt_hero.set_effect_description(
            "Alternate digivolution: Lv.5 with [Hero] trait, cost 5")
        alt_hero._alt_digi_cost = 5
        alt_hero._alt_digi_level = 5
        alt_hero._alt_digi_trait = "Hero"

        def cond_alt_hero(context: Dict[str, Any]) -> bool:
            return True
        alt_hero.set_can_use_condition(cond_alt_hero)
        effects.append(alt_hero)

        # ── Alt-Digi 2: From [Imperialdramon: Dragon Mode], cost 1 ────
        alt_dragon = ICardEffect()
        alt_dragon.set_effect_name("AD1-024 Alt-digi: Imperialdramon: Dragon Mode, cost 1")
        alt_dragon.set_effect_description(
            "Alternate digivolution: From [Imperialdramon: Dragon Mode], cost 1")
        alt_dragon._alt_digi_cost = 1
        alt_dragon._alt_digi_name = "Imperialdramon: Dragon Mode"

        def cond_alt_dragon(context: Dict[str, Any]) -> bool:
            return True
        alt_dragon.set_can_use_condition(cond_alt_dragon)
        effects.append(alt_dragon)

        # ── Security A. +1 ─────────────────────────────────────────────
        effect_sa = ICardEffect()
        effect_sa.set_effect_name("AD1-024 Security Attack +1")
        effect_sa.set_effect_description("Security Attack +1")
        effect_sa._security_attack_modifier = 1

        def cond_sa(context: Dict[str, Any]) -> bool:
            return True
        effect_sa.set_can_use_condition(cond_sa)
        effects.append(effect_sa)

        # ── Blocker ────────────────────────────────────────────────────
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("AD1-024 Blocker")
        effect_blocker.set_effect_description("Blocker")
        effect_blocker._is_blocker = True

        def cond_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(cond_blocker)
        effects.append(effect_blocker)

        # ── [When Digivolving] [When Attacking] [Once Per Turn] ────────
        # Return 1 of your opponent's lowest DP Digimon to the bottom of
        # the deck.
        # C# hash: "AD1_024_WD_WA", OPT(1)
        SHARED_WD_WA_HASH = "AD1_024_WD_WA"

        def _lowest_dp_filter(enemy):
            """Find opponent Digimon with lowest DP, return filter function."""
            opp_digimon = [p for p in enemy.battle_area
                           if p.is_digimon and p.dp is not None]
            if not opp_digimon:
                return None
            min_dp = min(p.dp for p in opp_digimon)

            def filt(p):
                return p.is_digimon and p.dp is not None and p.dp == min_dp
            return filt

        def _wd_wa_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            filt = _lowest_dp_filter(enemy)
            if filt is None:
                return

            def on_select(target_perm):
                enemy.return_permanent_to_deck_bottom(target_perm)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=filt, is_optional=False)

        # [When Digivolving] instance
        eff_wd = ICardEffect()
        eff_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        eff_wd.set_effect_name("AD1-024 Lowest DP bottom deck (When Digivolving)")
        eff_wd.set_effect_description(
            "[When Digivolving] [Once Per Turn] Return 1 of your opponent's "
            "lowest DP Digimon to the bottom of the deck.")
        eff_wd.is_when_digivolving = True
        eff_wd.set_max_count_per_turn(1)
        eff_wd.set_hash_string(SHARED_WD_WA_HASH)

        def cond_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            # Must have at least 1 opponent Digimon
            return any(p.is_digimon and p.dp is not None
                       for p in enemy.battle_area)
        eff_wd.set_can_use_condition(cond_wd)
        eff_wd.set_on_process_callback(_wd_wa_process)
        effects.append(eff_wd)

        # [When Attacking] instance
        eff_wa = ICardEffect()
        eff_wa.set_timing(EffectTiming.OnUseAttack)
        eff_wa.set_effect_name("AD1-024 Lowest DP bottom deck (When Attacking)")
        eff_wa.set_effect_description(
            "[When Attacking] [Once Per Turn] Return 1 of your opponent's "
            "lowest DP Digimon to the bottom of the deck.")
        eff_wa.is_on_attack = True
        eff_wa.set_max_count_per_turn(1)
        eff_wa.set_hash_string(SHARED_WD_WA_HASH)

        def cond_wa(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(p.is_digimon and p.dp is not None
                       for p in enemy.battle_area)
        eff_wa.set_can_use_condition(cond_wa)
        eff_wa.set_on_process_callback(_wd_wa_process)
        effects.append(eff_wa)

        # ── [All Turns] [Once Per Turn] Observer effect ────────────────
        # When Digimon are played or digivolve, you may suspend 1 of your
        # opponent's Digimon and unsuspend this Digimon.
        # Then, if played or digivolved by effects, you may return 1 of your
        # opponent's suspended Digimon to the bottom of the deck.
        #
        # This fires on ANY Digimon play/digivolve, not restricted to own.
        # C# hash: "AD1_024_AT", OPT(1)
        #
        # Implementation: use _is_play_observer + _is_digivolve_observer
        # to catch own-side events. For opponent-side events, use
        # OnEnterFieldAnyone timing (without is_on_play flag) which fires
        # on all permanents when any card enters the field.

        AT_HASH = "AD1_024_AT"

        def _at_process(ctx: Dict[str, Any], is_effect_triggered: bool):
            """All Turns process: suspend opp Digimon + unsuspend self.
            Then if by effect, bottom-deck suspended opp Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Step 1: May suspend 1 of opponent's Digimon
            def opp_digimon_filter(p):
                return p.is_digimon and not p.is_suspended

            def on_suspend_select(target_perm):
                target_perm.suspend()
                # Unsuspend self
                if perm and perm in player.battle_area:
                    perm.unsuspend()

                # Step 2: "Then, if played or digivolved by effects"
                if is_effect_triggered:
                    _do_bottom_deck_step(player, game, enemy)

            def on_skip_suspend():
                # Player declined to suspend (effect is optional)
                pass

            game.effect_select_opponent_permanent(
                player, on_suspend_select,
                filter_fn=opp_digimon_filter,
                is_optional=True,
                prompt="You may suspend 1 of your opponent's Digimon.")

        def _do_bottom_deck_step(player, game, enemy):
            """If played/digivolved by effects, return 1 suspended opp Digimon to deck bottom."""
            def suspended_filter(p):
                return p.is_digimon and p.is_suspended

            if not any(suspended_filter(p) for p in enemy.battle_area):
                return

            def on_bottom_deck(target_perm):
                enemy.return_permanent_to_deck_bottom(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bottom_deck,
                filter_fn=suspended_filter,
                is_optional=True,
                prompt="You may return 1 of your opponent's suspended "
                       "Digimon to the bottom of the deck.")

        # Play observer: fires when OWN Digimon are played
        eff_play_obs = ICardEffect()
        eff_play_obs.set_effect_name("AD1-024 All Turns observer (play)")
        eff_play_obs.set_effect_description(
            "[All Turns] [Once Per Turn] When Digimon are played or digivolve, "
            "you may suspend 1 of your opponent's Digimon and unsuspend this "
            "Digimon. Then, if played or digivolved by effects, you may return "
            "1 of your opponent's suspended Digimon to the bottom of the deck.")
        eff_play_obs.is_optional = True
        eff_play_obs._is_play_observer = True
        eff_play_obs.set_max_count_per_turn(1)
        eff_play_obs.set_hash_string(AT_HASH)

        def cond_play_obs(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            # Must be a Digimon entering
            played_perm = context.get('played_permanent')
            if not played_perm or not played_perm.is_digimon:
                return False
            # Must have at least 1 opponent Digimon to suspend
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(p.is_digimon and not p.is_suspended
                       for p in enemy.battle_area)
        eff_play_obs.set_can_use_condition(cond_play_obs)

        def proc_play_obs(ctx: Dict[str, Any]):
            _at_process(ctx, is_effect_triggered=ctx.get('is_effect_play', False))
        eff_play_obs.set_on_process_callback(proc_play_obs)
        effects.append(eff_play_obs)

        # Digivolve observer: fires when OWN Digimon digivolve
        eff_digi_obs = ICardEffect()
        eff_digi_obs.set_effect_name("AD1-024 All Turns observer (digivolve)")
        eff_digi_obs.set_effect_description(
            "[All Turns] [Once Per Turn] When Digimon are played or digivolve "
            "(digivolve observer).")
        eff_digi_obs.is_optional = True
        eff_digi_obs._is_digivolve_observer = True
        eff_digi_obs.set_max_count_per_turn(1)
        eff_digi_obs.set_hash_string(AT_HASH)

        def cond_digi_obs(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            digivolved_perm = context.get('digivolved_permanent')
            if not digivolved_perm or not digivolved_perm.is_digimon:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(p.is_digimon and not p.is_suspended
                       for p in enemy.battle_area)
        eff_digi_obs.set_can_use_condition(cond_digi_obs)

        def proc_digi_obs(ctx: Dict[str, Any]):
            _at_process(ctx, is_effect_triggered=ctx.get('is_effect_play', False))
        eff_digi_obs.set_on_process_callback(proc_digi_obs)
        effects.append(eff_digi_obs)

        # OnEnterFieldAnyone observer: fires when ANY Digimon is played
        # (including opponent's). Using OnEnterFieldAnyone without is_on_play
        # flag causes this to fire for ALL permanents, not just the entering card.
        eff_enter_obs = ICardEffect()
        eff_enter_obs.set_timing(EffectTiming.OnEnterFieldAnyone)
        eff_enter_obs.set_effect_name("AD1-024 All Turns observer (opponent play)")
        eff_enter_obs.set_effect_description(
            "[All Turns] [Once Per Turn] When Digimon are played (opponent-side).")
        eff_enter_obs.is_optional = True
        eff_enter_obs.set_max_count_per_turn(1)
        eff_enter_obs.set_hash_string(AT_HASH)

        def cond_enter_obs(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False

            # This fires for ALL permanents when OnEnterFieldAnyone triggers.
            # We only want to respond when a DIFFERENT permanent enters
            # (specifically an opponent's Digimon being played).
            played_perm = context.get('played_permanent')
            if not played_perm:
                return False
            if not played_perm.is_digimon:
                return False
            # Skip if this IS the played permanent (self-entering doesn't count
            # since the card's own On Play would handle it)
            if played_perm is perm:
                return False
            # Only fire for OPPONENT's play (own plays handled by play_observer)
            event_player = context.get('event_player')
            if event_player is owner:
                return False

            # Must have opponent Digimon to suspend
            return any(p.is_digimon and not p.is_suspended
                       for p in enemy.battle_area)
        eff_enter_obs.set_can_use_condition(cond_enter_obs)

        def proc_enter_obs(ctx: Dict[str, Any]):
            _at_process(ctx, is_effect_triggered=ctx.get('is_effect_play', False))
        eff_enter_obs.set_on_process_callback(proc_enter_obs)
        effects.append(eff_enter_obs)

        # WhenDigivolving observer: fires when ANY Digimon digivolves
        # (including opponent's). Using WhenDigivolving without
        # is_when_digivolving flag causes this to fire for ALL permanents.
        eff_digi_global = ICardEffect()
        eff_digi_global.set_timing(EffectTiming.WhenDigivolving)
        eff_digi_global.set_effect_name("AD1-024 All Turns observer (opponent digivolve)")
        eff_digi_global.set_effect_description(
            "[All Turns] [Once Per Turn] When Digimon digivolve (opponent-side).")
        eff_digi_global.is_optional = True
        eff_digi_global.set_max_count_per_turn(1)
        eff_digi_global.set_hash_string(AT_HASH)

        def cond_digi_global(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False

            digivolved_perm = context.get('digivolved_permanent')
            if not digivolved_perm:
                return False
            if not digivolved_perm.is_digimon:
                return False
            # Skip if self
            if digivolved_perm is perm:
                return False
            # Only fire for OPPONENT's digivolve (own handled by _is_digivolve_observer)
            # The digivolved_permanent is on a specific player's field
            # Check if it's on the enemy's field
            if digivolved_perm not in enemy.battle_area:
                return False

            return any(p.is_digimon and not p.is_suspended
                       for p in enemy.battle_area)
        eff_digi_global.set_can_use_condition(cond_digi_global)

        def proc_digi_global(ctx: Dict[str, Any]):
            _at_process(ctx, is_effect_triggered=ctx.get('is_effect_play', False))
        eff_digi_global.set_on_process_callback(proc_digi_global)
        effects.append(eff_digi_global)

        return effects
