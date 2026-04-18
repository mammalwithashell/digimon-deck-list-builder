from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_064(CardScript):
    """EX8-064 Boltboutamon | Lv.7

    [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon and,
    for the turn, all of their Digimon get -6000 DP. Then, if DNA
    digivolving, you may play up to 10 play cost's total worth of [NSo]
    trait Digimon cards from your trash without paying the cost.

    [All Turns] [Once Per Turn] When other Digimon are deleted, trash your
    opponent's top security card.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ─── Effect 1: [When Digivolving] (main activate class) ─────────
        # C# reference: EX8_064.cs ActivateClass in OnEnterFieldAnyone.
        # Combines: (1) select 1 opp Digimon → de-digivolve 3,
        #           (2) -6000 DP for turn to all opp Digimon,
        #           (3) if DNA digivolving: play NSo Digimon from trash,
        #               budget of 10 total play cost, free, optional.
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name(
            "EX8-064 De-Digivolve 3 + -6000 DP (+ DNA: NSo trash plays)"
        )
        effect_wd.set_effect_description(
            "[When Digivolving] <De-Digivolve 3> 1 of your opponent's "
            "Digimon and, for the turn, all of their Digimon get -6000 DP. "
            "Then, if DNA digivolving, you may play up to 10 play cost's "
            "total worth of [NSo] trait Digimon cards from your trash "
            "without paying the cost."
        )
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            # Field presence required
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect_wd.set_can_use_condition(condition_wd)

        def _apply_dp_debuff(player, game):
            """Register -6000 DP end_of_turn on every current opponent Digimon.

            Per-target CHANGE_DP modifier with a target-equality condition
            so each modifier only applies to its registered target, matching
            DCGO ChangeDigimonDPPlayerEffect semantics (aura scoped to the
            specific permanents that existed when the effect fired).
            """
            from ....interfaces.modifiers import ModifierType
            enemy = player.enemy if player else None
            if not enemy:
                return
            for target in list(enemy.battle_area):
                if not target.is_digimon:
                    continue
                game.register_modifier(
                    target,
                    ModifierType.CHANGE_DP,
                    condition=lambda p, ctx_, fp=target: p is fp,
                    value_fn=lambda dp, t, ctx_: dp - 6000,
                    expiry='end_of_turn',
                )

        def _offer_nso_trash_plays(player, game, remaining_budget):
            """Recursively offer the player NSo Digimon from trash to play
            (free) while their combined play cost stays within the budget.

            This implements the DCGO 'Select up to 10 play cost to play'
            loop: each pick deducts its cost from the remaining budget, and
            further picks are filtered by the new budget. The player may
            stop at any time (the selection is optional)."""
            if remaining_budget <= 0:
                return

            def nso_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('NSo' in t for t in traits):
                    return False
                cost = getattr(c, 'get_cost_itself', 0) or 0
                if cost > remaining_budget:
                    return False
                return True

            # Only request selection if there is at least one eligible card
            eligible_exists = any(nso_filter(c) for c in player.trash_cards)
            if not eligible_exists:
                return

            def on_played(played_card, _played_perm):
                cost = getattr(played_card, 'get_cost_itself', 0) or 0
                _offer_nso_trash_plays(player, game, remaining_budget - cost)

            game.effect_play_from_zone(
                player, 'trash', nso_filter,
                free=True, is_optional=True,
                prompt=(
                    f"Select an [NSo] Digimon from trash to play without "
                    f"paying the cost (budget: {remaining_budget})."
                ),
                on_played=on_played,
            )

        def process_wd(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            is_dna = bool(ctx.get('is_dna_digivolve'))

            # Step A: Select 1 opponent Digimon to de-digivolve.
            def on_target_selected(target_perm):
                if target_perm is None:
                    # Shouldn't happen (is_optional=False) but guard anyway
                    _apply_dp_debuff(player, game)
                    if is_dna:
                        _offer_nso_trash_plays(player, game, 10)
                    return
                removed = target_perm.de_digivolve(3)
                if removed and enemy:
                    # de_digivolve leaves trashing to the caller
                    enemy.trash_cards.extend(removed)
                # Step B: after de-digivolve, apply -6000 DP to all current
                # opponent Digimon.
                _apply_dp_debuff(player, game)
                # Step C: if DNA digivolving, offer the trash play loop.
                if is_dna:
                    _offer_nso_trash_plays(player, game, 10)

            has_target = any(
                p.is_digimon for p in enemy.battle_area
            ) if enemy else False

            if has_target:
                game.effect_select_opponent_permanent(
                    player, on_target_selected,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to De-Digivolve 3.",
                )
            else:
                # No de-digivolve target available — still apply DP debuff
                # and DNA trash play (DCGO runs both unconditionally after
                # the optional target step).
                _apply_dp_debuff(player, game)
                if is_dna:
                    _offer_nso_trash_plays(player, game, 10)

        effect_wd.set_on_process_callback(process_wd)
        effects.append(effect_wd)

        # ─── Effect 2: [All Turns] [Once Per Turn] Deletion observer ────
        # C# reference: ActivateClass OnDestroyedAnyone, but filtered via
        # TriggerPermanentCondition where permanent != self. In the Python
        # engine, 'self-death' effects use is_on_deletion; watchers of
        # OTHER deletions use _is_deletion_observer + NoTiming so
        # _fire_deletion_observers scans battle_area for the effect.
        effect_obs = ICardEffect()
        effect_obs.set_effect_name(
            "EX8-064 Trash your opponent's top security card"
        )
        effect_obs.set_effect_description(
            "[All Turns] [Once Per Turn] When other Digimon are deleted, "
            "trash your opponent's top security card."
        )
        effect_obs._is_deletion_observer = True
        effect_obs.set_max_count_per_turn(1)
        effect_obs.set_hash_string("TrashSecurity_EX8_064")

        def condition_obs(context: Dict[str, Any]) -> bool:
            # Field presence required
            if card and card.permanent_of_this_card() is None:
                return False
            deleted_perm = context.get('deleted_permanent')
            if not deleted_perm:
                return False
            # Only "other Digimon" — must be a Digimon, not this card's own
            # permanent.
            if not getattr(deleted_perm, 'is_digimon', False):
                return False
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm is not None and deleted_perm is own_perm:
                return False
            return True

        effect_obs.set_can_use_condition(condition_obs)

        def process_obs(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player is None and card:
                player = card.owner
            if not player:
                return
            enemy = player.enemy
            if not enemy:
                return
            if enemy.security_cards:
                top_sec = enemy.security_cards[0]
                enemy.trash_security_card(top_sec)

        effect_obs.set_on_process_callback(process_obs)
        effects.append(effect_obs)

        return effects
