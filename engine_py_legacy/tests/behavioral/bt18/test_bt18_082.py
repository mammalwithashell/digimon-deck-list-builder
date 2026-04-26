"""Behavioral tests for BT18-082 Lucemon: Chaos Mode (Lv.5 Purple/Yellow Demon Lord).

Card text:
  [On Play] [When Digivolving] Your opponent may delete 1 of their Digimon or
    Tamers. If this effect didn't delete, <Recovery +1 (Deck)> (Place the top
    card of your deck on top of your security stack.) and trash the top card
    of your opponent's security stack.
  [All Turns] [Once Per Turn] When this Digimon would leave the battle area,
    by trashing the bottom card of your security stack, it doesn't leave.

Alt-digi: <Digivolve> [Lucemon]: Cost 6 (from plain "Lucemon" at Lv.3).

Key faithfulness points tested:
  - C1 [On Play] and C2 [When Digivolving] are SEPARATE effects.
  - C1/C2 selection is given to the OPPONENT (they pick their own Digimon/Tamer).
  - If opponent has no valid targets: Recovery + trash opp sec top fires.
  - If opponent attempts and deletion succeeds: NO recovery/trash.
  - If opponent declines optional selection: Recovery + trash opp sec top fires.
  - Tamers are valid delete targets.
  - C3 WhenPermanentWouldBeDeleted with OPT once-per-turn.
  - C3 cost is trashing the BOTTOM security card (index -1).
  - C3 sets `_will_not_be_removed = True` to prevent deletion.
  - C3 only fires for THIS Digimon (event_permanent self-check).
  - Alt-digi effect exists (Cost 6 from plain Lucemon).
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming, GamePhase


LUCEMON_CM = "BT18-082"     # Lucemon: Chaos Mode (Lv.5 target)
LUCEMON_BASE = "BT4-115"    # plain "Lucemon" (Lv.3 Yellow — for alt-digi)
AGUMON = "ST1-03"           # Lv.3 Red Digimon filler
WARGREYMON = "ST1-11"       # Lv.6 Red Digimon (high DP)
TAMER = "ST1-12"            # Tai Kamiya (Tamer)

FILLER_DECK = [AGUMON] * 50


@pytest.mark.behavioral
class TestBT18082Structure:
    """Structural checks for BT18-082 effects."""

    def test_has_on_play_effect(self, debug_runner):
        """Should have exactly 1 On Play effect (is_on_play=True)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        effects = perm.top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) == 1, (
            f"Should have exactly 1 On Play effect, got {len(on_play)}"
        )

    def test_has_when_digivolving_effect(self, debug_runner):
        """Should have exactly 1 When Digivolving effect (is_when_digivolving=True)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        effects = perm.top_card.effect_list(None)
        when_digi = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(when_digi) == 1, (
            f"Should have exactly 1 When Digivolving effect, got {len(when_digi)}"
        )

    def test_on_play_and_when_digivolving_are_separate_instances(self, debug_runner):
        """C1 and C2 must be separate ICardEffect instances."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        effects = perm.top_card.effect_list(None)
        enter_field_effects = [
            e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone
        ]
        on_play = [e for e in enter_field_effects if getattr(e, 'is_on_play', False)]
        when_digi = [
            e for e in enter_field_effects if getattr(e, 'is_when_digivolving', False)
        ]
        assert len(on_play) == 1 and len(when_digi) == 1
        # Must be distinct objects
        assert on_play[0] is not when_digi[0], (
            "On Play and When Digivolving must be separate ICardEffect instances"
        )

    def test_has_leave_prevention_effect(self, debug_runner):
        """Should have a WhenPermanentWouldBeDeleted effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        effects = perm.top_card.effect_list(None)
        leave_effects = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ]
        assert len(leave_effects) == 1, (
            f"Should have exactly 1 WhenPermanentWouldBeDeleted effect, got {len(leave_effects)}"
        )

    def test_leave_prevention_is_opt(self, debug_runner):
        """Leave prevention must be OPT (max_count_per_turn=1)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        effects = perm.top_card.effect_list(None)
        leave = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]
        assert leave.max_count_per_turn == 1, (
            f"Leave prevention must be OPT (max_count_per_turn=1), got {leave.max_count_per_turn}"
        )
        assert leave.is_optional, "Leave prevention must be optional"

    def test_alt_digi_from_lucemon_cost_6(self, debug_runner):
        """Alt-digi requirement: from plain 'Lucemon' at Lv.3 for cost 6."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        runner.inject_card(1, LUCEMON_CM, "hand")
        game = runner.game
        cm_card = [
            c for c in game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == LUCEMON_CM
        ][0]
        effects = cm_card.effect_list(None)
        alt_digi = [
            e for e in effects
            if getattr(e, '_alt_digi_cost', None) is not None
        ]
        assert len(alt_digi) >= 1, (
            f"Should have at least 1 alt-digi effect, got {len(alt_digi)}"
        )
        # Find the one with cost 6 and Lucemon name/condition
        matching = [e for e in alt_digi if e._alt_digi_cost == 6]
        assert len(matching) >= 1, (
            f"Should have alt-digi with cost 6, got "
            f"{[e._alt_digi_cost for e in alt_digi]}"
        )


@pytest.mark.behavioral
class TestBT18082OnPlayOpponentDeletes:
    """C1 [On Play] — opponent may delete, else Recovery+trash opp sec top."""

    def _fire_on_play(self, runner, perm):
        """Fire the OnEnterFieldAnyone timing for the given permanent."""
        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "permanent": perm,
            "player": runner.game.player1,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })

    def test_on_play_no_opp_targets_triggers_recovery_and_sec_trash(self, debug_runner):
        """With no opp Digimon/Tamers, Recovery +1 and trash opp sec top both fire."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [LUCEMON_CM])
        # Ensure P2 has no Digimon/Tamers on field
        assert len(runner.game.player2.battle_area) == 0

        p1_sec_before = len(runner.game.player1.security_cards)
        p1_deck_before = len(runner.game.player1.library_cards)
        p2_sec_before = len(runner.game.player2.security_cards)
        p2_trash_before = len(runner.game.player2.trash_cards)

        self._fire_on_play(runner, perm)
        runner.auto_resolve()

        # Recovery +1: P1 security +1 (or =original if unchanged — check deck size)
        assert len(runner.game.player1.library_cards) == p1_deck_before - 1, (
            "Recovery should remove 1 card from P1's deck"
        )
        assert len(runner.game.player1.security_cards) == p1_sec_before + 1, (
            f"Recovery should add 1 to P1's security, before={p1_sec_before}, "
            f"after={len(runner.game.player1.security_cards)}"
        )
        # Opp sec trashed: P2 security -1, P2 trash +1
        assert len(runner.game.player2.security_cards) == p2_sec_before - 1, (
            f"Opp security should lose 1 card, before={p2_sec_before}, "
            f"after={len(runner.game.player2.security_cards)}"
        )
        assert len(runner.game.player2.trash_cards) == p2_trash_before + 1, (
            "Opp trash should gain 1 card"
        )

    def test_on_play_opponent_selects_to_delete_skips_recovery(self, debug_runner):
        """With an opp Digimon, opponent selects to delete → NO recovery/trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [LUCEMON_CM])
        target_perm = runner.place_on_field(2, [AGUMON])

        p1_sec_before = len(runner.game.player1.security_cards)
        p1_deck_before = len(runner.game.player1.library_cards)
        p2_sec_before = len(runner.game.player2.security_cards)
        p2_trash_before = len(runner.game.player2.trash_cards)

        self._fire_on_play(runner, perm)

        # Opponent (P2) is now selecting. Pick the first valid target
        # (not action 62 = decline). Valid indices start at SEL_MY_FIELD_START=100.
        ps = runner.game.pending_selection
        assert ps is not None
        assert ps.selecting_player is runner.game.player2
        target_action = ps.valid_indices[0]
        runner.game.decode_action(target_action, runner.game.current_player_id)
        runner.auto_resolve()

        # Opponent's Digimon should be deleted
        assert target_perm not in runner.game.player2.battle_area, (
            "Opponent's selected Digimon should be deleted"
        )
        # P1 recovery should NOT fire
        assert len(runner.game.player1.library_cards) == p1_deck_before, (
            "P1 deck should not shrink (no recovery)"
        )
        assert len(runner.game.player1.security_cards) == p1_sec_before, (
            "P1 security should not grow (no recovery)"
        )
        # Opp security should NOT be trashed
        assert len(runner.game.player2.security_cards) == p2_sec_before, (
            "Opp security should not shrink (delete path = no trash)"
        )

    def test_on_play_opponent_can_select_tamer_target(self, debug_runner):
        """Opponent can select a Tamer as the delete target."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [LUCEMON_CM])
        # Place a single Tamer on opp field (no Digimon)
        tamer_perm = runner.place_on_field(2, [TAMER])
        assert tamer_perm.is_tamer

        self._fire_on_play(runner, perm)

        # P2 should see the Tamer as a valid target. Pick the first one.
        ps = runner.game.pending_selection
        assert ps is not None, "Pending selection should exist for opponent"
        assert len(ps.valid_indices) >= 1, (
            f"Tamer should appear as valid target, got {ps.valid_indices}"
        )
        target_action = ps.valid_indices[0]
        runner.game.decode_action(target_action, runner.game.current_player_id)
        runner.auto_resolve()

        # Tamer should be deletable (valid target)
        assert tamer_perm not in runner.game.player2.battle_area, (
            "Opponent's Tamer should be a valid delete target"
        )

    def test_on_play_selection_is_opponent_side(self, debug_runner):
        """When selection phase opens, active_player must be P2 (opponent)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [LUCEMON_CM])
        runner.place_on_field(2, [AGUMON])
        runner.place_on_field(2, [TAMER])

        self._fire_on_play(runner, perm)

        # After firing, the game should be in SelectTarget phase with P2 as active
        ps = runner.game.pending_selection
        assert ps is not None, "Pending selection should exist"
        assert ps.selecting_player is runner.game.player2, (
            f"Opponent (P2) should be the selecting player, "
            f"got {ps.selecting_player.player_name if ps.selecting_player else None}"
        )
        # is_optional should be True ("your opponent MAY delete")
        assert ps.is_optional, "Selection should be optional (opponent MAY delete)"

    def test_on_play_opponent_targets_are_only_own(self, debug_runner):
        """Valid indices in pending selection only point to P2's own battle_area."""
        from digimon_gym.engine.game.constants import (
            SEL_MY_FIELD_START, SEL_OPP_FIELD_START,
        )
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [LUCEMON_CM])
        # Put Digimon on BOTH sides
        runner.place_on_field(1, [AGUMON])  # P1's own — should NOT be selectable by P2
        runner.place_on_field(2, [AGUMON])  # P2's own — should be selectable
        runner.place_on_field(2, [TAMER])

        self._fire_on_play(runner, perm)

        ps = runner.game.pending_selection
        assert ps is not None
        # From P2's perspective, selecting "own" permanents = SEL_MY_FIELD_START range
        for idx in ps.valid_indices:
            assert SEL_MY_FIELD_START <= idx < SEL_MY_FIELD_START + 14, (
                f"Valid index {idx} should be in own-field range (P2's own), "
                f"not opponent-field range"
            )
        # Should be exactly 2 targets (P2's Agumon + P2's Tamer)
        assert len(ps.valid_indices) == 2, (
            f"Should be 2 valid targets (P2's Agumon + Tamer), got {len(ps.valid_indices)}"
        )

    def test_on_play_opponent_declines_triggers_recovery_and_sec_trash(self, debug_runner):
        """If opponent declines (action 62), the Recovery+trash path fires."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [LUCEMON_CM])
        runner.place_on_field(2, [AGUMON])

        p1_sec_before = len(runner.game.player1.security_cards)
        p1_deck_before = len(runner.game.player1.library_cards)
        p2_sec_before = len(runner.game.player2.security_cards)
        p2_battle_before = len(runner.game.player2.battle_area)

        self._fire_on_play(runner, perm)

        # Decline: action 62 while in SelectTarget phase
        ps = runner.game.pending_selection
        assert ps is not None
        assert ps.is_optional
        runner.game.decode_action(62, runner.game.current_player_id)
        runner.auto_resolve()

        # Opponent's Agumon should still be on field (not deleted)
        assert len(runner.game.player2.battle_area) == p2_battle_before, (
            "Opponent declined — Digimon should NOT be deleted"
        )
        # Recovery should fire
        assert len(runner.game.player1.library_cards) == p1_deck_before - 1, (
            "Recovery should fire when opponent declines"
        )
        assert len(runner.game.player1.security_cards) == p1_sec_before + 1, (
            "P1 security should grow by 1"
        )
        # Opp sec top trashed
        assert len(runner.game.player2.security_cards) == p2_sec_before - 1, (
            "Opp security should lose 1 card when opponent declines"
        )

    def test_on_play_no_opp_security_still_recovers(self, debug_runner):
        """With empty opp security, Recovery still fires; only trash is skipped."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [LUCEMON_CM])
        runner.clear_zone(2, "security")
        assert len(runner.game.player2.security_cards) == 0

        p1_sec_before = len(runner.game.player1.security_cards)
        p1_deck_before = len(runner.game.player1.library_cards)

        self._fire_on_play(runner, perm)
        runner.auto_resolve()

        # Recovery should still happen
        assert len(runner.game.player1.library_cards) == p1_deck_before - 1
        assert len(runner.game.player1.security_cards) == p1_sec_before + 1


@pytest.mark.behavioral
class TestBT18082WhenDigivolving:
    """C2 [When Digivolving] — same flow as On Play."""

    def test_when_digivolving_fires_no_targets_trash_recovery(self, debug_runner):
        """When Digivolving with no opp targets: Recovery + trash opp sec top."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Stack: Lucemon (Lv.3) → Lucemon CM (Lv.5) = digivolved stack
        perm = runner.place_on_field(1, [LUCEMON_BASE, LUCEMON_CM])
        # No opp targets
        assert len(runner.game.player2.battle_area) == 0

        p1_sec_before = len(runner.game.player1.security_cards)
        p2_sec_before = len(runner.game.player2.security_cards)

        runner.game.execute_effects(EffectTiming.WhenDigivolving, {
            "digivolved_permanent": perm,
            "permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        # Recovery fires
        assert len(runner.game.player1.security_cards) == p1_sec_before + 1, (
            "When Digivolving path should Recovery +1 with no opp targets"
        )
        # Opp sec trashed
        assert len(runner.game.player2.security_cards) == p2_sec_before - 1, (
            "When Digivolving path should trash opp sec top with no opp targets"
        )

    def test_when_digivolving_opponent_deletes_skips_recovery(self, debug_runner):
        """When Digivolving with opp target: opponent deletes, no recovery."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [LUCEMON_BASE, LUCEMON_CM])
        target_perm = runner.place_on_field(2, [AGUMON])

        p1_sec_before = len(runner.game.player1.security_cards)
        p2_sec_before = len(runner.game.player2.security_cards)

        runner.game.execute_effects(EffectTiming.WhenDigivolving, {
            "digivolved_permanent": perm,
            "permanent": perm,
            "player": runner.game.player1,
        })

        # Opponent selects to delete their own
        ps = runner.game.pending_selection
        assert ps is not None, "Pending selection should open for opponent"
        target_action = ps.valid_indices[0]
        runner.game.decode_action(target_action, runner.game.current_player_id)
        runner.auto_resolve()

        assert target_perm not in runner.game.player2.battle_area
        assert len(runner.game.player1.security_cards) == p1_sec_before, (
            "No recovery when opp deletes their own"
        )
        assert len(runner.game.player2.security_cards) == p2_sec_before, (
            "No trash when opp deletes their own"
        )


@pytest.mark.behavioral
class TestBT18082LeavePrevention:
    """C3 [All Turns] [Once Per Turn] — trash bottom security to prevent leaving."""

    def test_c3_self_identity_check_in_condition(self, debug_runner):
        """C3 should only fire when event_permanent is THIS Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        other_perm = runner.place_on_field(1, [AGUMON])

        top = perm.top_card
        effects = top.effect_list(None)
        leave = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        # When event_permanent is OTHER perm, condition should fail
        ctx_other = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": perm,  # host (scanned)
            "event_permanent": other_perm,  # the one being deleted
            "card": top,
        }
        assert not leave.can_use_condition(ctx_other), (
            "C3 should NOT fire when a different permanent would leave"
        )

        # When event_permanent is THIS perm, condition should pass
        ctx_self = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": perm,
            "event_permanent": perm,
            "card": top,
        }
        assert leave.can_use_condition(ctx_self), (
            "C3 should fire when THIS permanent would leave"
        )

    def test_c3_prevents_deletion_via_trash_bottom_security(self, debug_runner):
        """Calling delete_permanent → C3 fires → bottom sec trashed → perm survives."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        game = runner.game

        # Grab references to top & bottom of security (index 0 = top, -1 = bottom)
        assert len(game.player1.security_cards) >= 2
        sec_top_before = game.player1.security_cards[0]
        sec_bottom_before = game.player1.security_cards[-1]
        sec_count_before = len(game.player1.security_cards)
        p1_trash_before = len(game.player1.trash_cards)

        # Call delete — this fires WhenPermanentWouldBeDeleted
        result = game.player1.delete_permanent(perm)
        runner.auto_resolve()

        # Deletion prevented
        assert result is False, "delete_permanent should return False (prevented)"
        assert perm in game.player1.battle_area, (
            "Lucemon CM should survive deletion"
        )
        # Security: lost 1 (bottom)
        assert len(game.player1.security_cards) == sec_count_before - 1, (
            f"Should lose 1 security card, before={sec_count_before}, "
            f"after={len(game.player1.security_cards)}"
        )
        # Top still there
        assert sec_top_before in game.player1.security_cards, (
            "Top security card should NOT be trashed"
        )
        # Bottom in trash now
        assert sec_bottom_before in game.player1.trash_cards, (
            "Bottom security card should be in trash"
        )
        assert sec_bottom_before not in game.player1.security_cards
        assert len(game.player1.trash_cards) == p1_trash_before + 1, (
            "P1 trash should gain 1 card"
        )

    def test_c3_trashes_bottom_not_top(self, debug_runner):
        """Ensure bottom (index -1) is trashed, NOT top (index 0)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        game = runner.game

        sec_top = game.player1.security_cards[0]
        sec_bottom = game.player1.security_cards[-1]
        assert sec_top is not sec_bottom, "Security stack must have distinct top/bottom"

        game.player1.delete_permanent(perm)
        runner.auto_resolve()

        # Top must still be in security, bottom must be in trash
        assert sec_top in game.player1.security_cards
        assert sec_top not in game.player1.trash_cards
        assert sec_bottom in game.player1.trash_cards
        assert sec_bottom not in game.player1.security_cards

    def test_c3_no_security_cannot_prevent(self, debug_runner):
        """With 0 security, C3 cannot pay cost → deletion proceeds."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        runner.clear_zone(1, "security")
        assert len(runner.game.player1.security_cards) == 0

        result = runner.game.player1.delete_permanent(perm)
        runner.auto_resolve()

        # Deletion should succeed
        assert result is True, "With no security, deletion should proceed"
        assert perm not in runner.game.player1.battle_area

    def test_c3_opt_once_per_turn(self, debug_runner):
        """C3 should be flagged OPT — subsequent activations blocked by _turn_activate_count."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [LUCEMON_CM])

        effects = perm.top_card.effect_list(None)
        leave = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]
        assert leave.max_count_per_turn == 1
        # Simulate one activation
        leave.record_activation()
        assert not leave.can_activate_this_turn(), (
            "After 1 activation, OPT should block further activations this turn"
        )

    def test_c3_has_hash_string(self, debug_runner):
        """C3 should have a unique hash_string for OPT tracking."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [LUCEMON_CM])
        effects = perm.top_card.effect_list(None)
        leave = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]
        assert leave.hash_string, (
            "C3 should have a hash_string set for OPT tracking"
        )
        assert "BT18" in leave.hash_string and "082" in leave.hash_string, (
            f"hash_string should identify BT18-082, got '{leave.hash_string}'"
        )

    def test_c3_field_presence_check(self, debug_runner):
        """C3 condition should fail when card is not on field (no host)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        runner.inject_card(1, LUCEMON_CM, "hand")
        game = runner.game

        hand_card = [
            c for c in game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == LUCEMON_CM
        ][0]
        effects = hand_card.effect_list(None)
        leave = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]
        # In hand → permanent_of_this_card() is None → condition False
        assert not leave.can_use_condition({}), (
            "C3 condition should be False when Lucemon CM is not on field"
        )
