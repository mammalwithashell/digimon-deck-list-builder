from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_031(CardScript):
    """EX10-031 DarkKnightmon | Lv.5 Black/Purple [Dark Knight/Bagra Army/Twilight]

    [On Play] [When Digivolving] Until your opponent's turn ends, their
    <De-Digivolve> effects don't affect 1 of your Digimon, and it gets +3000 DP.

    [All Turns] [Once Per Turn] When this Digimon would leave the battle area,
    you may play 1 play cost 4 or lower card from its digivolution cards
    without paying the cost.

    Inherited: [Opponent's Turn] [Once Per Turn] When one of your opponent's
    Digimon attacks, you may change the attack target to this Digimon.

    Alt-digi: From a Lv.4 Digimon with "Knightmon" in its card text, cost 3.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects: List['ICardEffect'] = []

        # ── Alt-Digi: Lv.4 Digimon with 'Knightmon' in card text, cost 3 ──
        effect_alt = ICardEffect()
        effect_alt.set_effect_name("EX10-031 Alt-digi: Lv.4 Knightmon-text, cost 3")
        effect_alt.set_effect_description(
            "Alternate digivolution: Lv.4 Digimon with 'Knightmon' in card text, cost 3"
        )
        effect_alt._alt_digi_cost = 3
        effect_alt._alt_digi_level = 4
        effect_alt._alt_digi_has_text = "Knightmon"

        def condition_alt(context: Dict[str, Any]) -> bool:
            return True

        effect_alt.set_can_use_condition(condition_alt)
        effects.append(effect_alt)

        # ── Shared: select a Digimon, grant immunity + +3000 DP ──
        def _select_and_grant(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_target(target_perm):
                if target_perm is None:
                    return
                # Grant IMMUNE_FROM_DE_DIGIVOLVE until end of opponent turn
                game.register_modifier(
                    target_perm,
                    ModifierType.IMMUNE_FROM_DE_DIGIVOLVE,
                    value_fn=lambda: True,
                    condition=lambda t, c, _tp=target_perm: t is _tp,
                    expiry='end_of_opponent_turn',
                )
                # Grant +3000 DP until end of opponent turn
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda cur, t, c: cur + 3000,
                    condition=lambda t, c, _tp=target_perm: t is _tp,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_own_permanent(
                player, on_target, filter_fn=target_filter,
                is_optional=False,
                prompt="Select 1 of your Digimon to gain <De-Digivolve> "
                       "immunity and +3000 DP until end of opponent's turn."
            )

        # ── C1 [On Play] De-Digi immunity + +3000 DP ──
        effect_op = ICardEffect()
        effect_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op.set_effect_name("EX10-031 [On Play] De-Digivolve immunity & +3000 DP")
        effect_op.set_effect_description(
            "[On Play] Until your opponent's turn ends, their <De-Digivolve> "
            "effects don't affect 1 of your Digimon, and it gets +3000 DP."
        )
        effect_op.is_on_play = True

        def condition_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect_op.set_can_use_condition(condition_op)
        effect_op.set_on_process_callback(_select_and_grant)
        effects.append(effect_op)

        # ── C2 [When Digivolving] De-Digi immunity + +3000 DP ──
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("EX10-031 [When Digivolving] De-Digivolve immunity & +3000 DP")
        effect_wd.set_effect_description(
            "[When Digivolving] Until your opponent's turn ends, their "
            "<De-Digivolve> effects don't affect 1 of your Digimon, and "
            "it gets +3000 DP."
        )
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect_wd.set_can_use_condition(condition_wd)
        effect_wd.set_on_process_callback(_select_and_grant)
        effects.append(effect_wd)

        # ── C3 [All Turns][Once Per Turn] When would leave: play from digi ──
        effect_leave = ICardEffect()
        effect_leave.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect_leave.set_effect_name("EX10-031 Play cost-4 Digimon from digivolution cards")
        effect_leave.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon would leave the "
            "battle area, you may play 1 play cost 4 or lower card from its "
            "digivolution cards without paying the cost."
        )
        effect_leave.is_optional = True
        effect_leave.set_max_count_per_turn(1)
        effect_leave.set_hash_string("PlayDigimon_EX10_031")

        def condition_leave(context: Dict[str, Any]) -> bool:
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm is None:
                return False
            # Only trigger when THIS perm is the one being deleted
            leaving = context.get('event_permanent') or context.get('permanent')
            if leaving is not own_perm:
                return False
            # Must have at least one qualifying Digimon card in the digi cards
            candidates = [
                cs for cs in own_perm.card_sources
                if cs is not own_perm.top_card
                and getattr(cs, 'is_digimon', False)
                and getattr(cs, 'has_play_cost', False)
                and getattr(cs, 'get_cost_itself', 0) <= 4
            ]
            return len(candidates) >= 1

        effect_leave.set_can_use_condition(condition_leave)

        def process_leave(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm is None:
                return

            candidates = [
                cs for cs in own_perm.card_sources
                if cs is not own_perm.top_card
                and getattr(cs, 'is_digimon', False)
                and getattr(cs, 'has_play_cost', False)
                and getattr(cs, 'get_cost_itself', 0) <= 4
            ]
            if not candidates:
                return

            # Cancel this deletion (set _will_not_be_removed) so the host perm
            # stays on the field during the selection. After the player resolves
            # (or declines) the selection, we re-issue delete_permanent to
            # complete the deletion that was originally triggered.
            own_perm._will_not_be_removed = True
            removal_cause = ctx.get('removal_cause', 'effect')
            is_opponent_effect = ctx.get('is_opponent_effect', False)
            is_battle = ctx.get('is_battle', False)

            # Find field index of the host permanent (for SelectSource action id)
            from ....game.constants import SOURCES_PER_FIELD
            field_idx = None
            for fi, fp in enumerate(player.battle_area):
                if fp is own_perm:
                    field_idx = fi
                    break
            if field_idx is None:
                return
            base = 2000 + field_idx * SOURCES_PER_FIELD
            valid = []
            for i, cs in enumerate(own_perm.card_sources):
                if cs in candidates and (base + i) < 2168:
                    valid.append(base + i)
            if not valid:
                return

            def _complete_deletion():
                """Re-issue the deletion that was cancelled to let the effect resolve."""
                # Only delete if still on the battle area (defensive)
                if own_perm in player.battle_area:
                    player.delete_permanent(
                        own_perm,
                        is_battle=is_battle,
                        is_opponent_effect=is_opponent_effect,
                        removal_cause=removal_cause,
                    )

            def on_source_selected(action_id: int):
                idx = action_id - base
                if not (0 <= idx < len(own_perm.card_sources)):
                    _complete_deletion()
                    return
                chosen = own_perm.card_sources[idx]
                # Only valid candidates (cost <= 4 Digimon) can be played
                if chosen not in candidates:
                    _complete_deletion()
                    return
                # Remove chosen from stack, then play as new permanent
                own_perm.card_sources.remove(chosen)
                played_perm = player.play_card_from_source(chosen, pay_cost=False)
                # Fire OnEnterFieldAnyone observers for the new permanent
                if played_perm is not None:
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": chosen,
                         "played_permanent": played_perm,
                         "event_player": player,
                         "is_effect_play": True},
                    )
                # Re-run deletion for the host perm now that the effect resolved
                _complete_deletion()

            game.request_selection(
                GamePhase.SelectSource, player, on_source_selected,
                valid, is_optional=True,
                prompt="Select a play cost 4 or lower Digimon card from this "
                       "permanent's digivolution cards to play.",
                on_decline=_complete_deletion,
            )

        effect_leave.set_on_process_callback(process_leave)
        effects.append(effect_leave)

        # ── C4 Inherited [Opponent's Turn][Once Per Turn] Redirect attack ──
        # Uses _is_when_attacked_observer pattern + game.switch_attack_target()
        effect_redir = ICardEffect()
        effect_redir.set_effect_name("EX10-031 Redirect attack to this Digimon")
        effect_redir.set_effect_description(
            "[Opponent's Turn] [Once Per Turn] When one of your opponent's "
            "Digimon attacks, you may change the attack target to this Digimon."
        )
        effect_redir.is_inherited_effect = True
        effect_redir.is_optional = True
        effect_redir.set_max_count_per_turn(1)
        effect_redir.set_hash_string("EX10_031_ChangeAttackTarget")
        effect_redir._is_when_attacked_observer = True

        def condition_redir(context: Dict[str, Any]) -> bool:
            # Must be on field
            host_perm = card.permanent_of_this_card() if card else None
            if host_perm is None:
                return False
            # Must be opponent's turn (we are NOT the turn player)
            owner = card.owner if card else None
            if owner is None or owner.is_my_turn:
                return False
            # Must be in an attack context with an opponent attacker
            game = context.get('game')
            attacker = context.get('attacker')
            if game is None or attacker is None:
                return False
            pa = getattr(game, 'pending_attack', None)
            if pa is None:
                # May be called pre-attack (PendingAttack not yet set) — allow
                pass
            # Host perm and attacker must not be the same side
            if getattr(attacker, 'is_digimon', False) is False:
                return False
            # Confirm attacker is on opponent's field
            if attacker in owner.battle_area:
                return False
            return True

        effect_redir.set_can_use_condition(condition_redir)

        def process_redir(ctx: Dict[str, Any]):
            game = ctx.get('game')
            host_perm = card.permanent_of_this_card() if card else None
            if not (game and host_perm):
                return
            # Switch attack target to host permanent (this Digimon)
            if hasattr(game, 'switch_attack_target'):
                game.switch_attack_target(host_perm)
            elif hasattr(game, 'redirect_attack'):
                game.redirect_attack(host_perm)

        effect_redir.set_on_process_callback(process_redir)
        effects.append(effect_redir)

        return effects
