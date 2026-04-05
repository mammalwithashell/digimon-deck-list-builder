"""Behavioral tests for BT20-017 Jesmon (Lv.6 Red Digimon).

Card text:
  [On Play] [When Digivolving] You may play 1 [Atho, Rene & Por] Token.
  (Digimon/White/6000 DP/<Reboot>/<Blocker>/<Decoy (Red/Black)>)
  [Your Turn] [Once Per Turn] When any of your other Digimon are played,
  delete 1 of your opponent's Digimon with 8000 DP or less.
  Then, 1 of your Digimon may attack.

Clause breakdown:
  C1: [On Play] - optional token play
  C2: [When Digivolving] - optional token play
  C3: [Your Turn][OPT] - trigger on other owned Digimon played
  C3a: delete opponent's Digimon with DP <= 8000 (mandatory when targets exist)
  C3b: Then, 1 of your Digimon may attack (optional, uses MAY_ATTACK)
"""

import pytest

from digimon_gym.engine.data.enums import GamePhase


@pytest.mark.behavioral
class TestBT20017Jesmon:
    """Tests for BT20-017 Jesmon."""

    # ── C1: [On Play] token play ───────────────────────────────────────

    def test_on_play_creates_token(self, debug_runner):
        """[On Play] should offer to play an Atho, Rene & Por Token."""
        runner = debug_runner(initial_memory=15)
        runner.inject_card(1, "BT20-017", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Jesmon")
        assert play is not None, (
            f"Should be able to play Jesmon. Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Jesmon should be on field
        jesmon = next((s for s in snap.p1_field if s.card_id == "BT20-017"), None)
        assert jesmon is not None, "Jesmon should be on the field"

        # Token should be on field
        token = next(
            (s for s in snap.p1_field if "Atho" in s.card_name or "TOKEN" in s.card_id),
            None)
        assert token is not None, (
            f"Atho, Rene & Por Token should be on field. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}")

    def test_on_play_token_is_optional(self, debug_runner):
        """[On Play] token play is optional -- declining should still work."""
        runner = debug_runner(initial_memory=15)
        runner.inject_card(1, "BT20-017", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Jesmon")
        assert play is not None
        runner.execute(play)

        # If there's a pending selection for the token, decline it
        snap = runner.snapshot()
        if snap.phase not in ("Main", "Breeding"):
            # There should be a decline option
            decline = runner.find_action("Decline")
            if decline is not None:
                runner.execute(decline)
                runner.auto_resolve()

        snap = runner.snapshot()
        jesmon = next((s for s in snap.p1_field if s.card_id == "BT20-017"), None)
        assert jesmon is not None, "Jesmon should be on the field even without token"

    # ── C3: [Your Turn][OPT] trigger on other Digimon played ──────────

    def test_your_turn_trigger_deletes_opponent_digimon(self, debug_runner):
        """When another owned Digimon is played, should delete opponent's
        Digimon with DP <= 8000."""
        runner = debug_runner(initial_memory=15)
        # Place Jesmon on field (already in play)
        runner.place_on_field(1, ["BT20-017"])
        # Place opponent Digimon with DP <= 8000
        runner.place_on_field(2, ["BT1-010"])  # Agumon, DP 2000

        # Play another Digimon from hand to trigger
        runner.inject_card(1, "ST1-03", "hand")  # Agumon Lv.3
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        assert play is not None, (
            f"Should be able to play Agumon. Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's Agumon should be deleted
        opp_agumon = next(
            (s for s in snap.p2_field if s.card_id == "BT1-010"), None)
        assert opp_agumon is None, (
            f"Opponent's Agumon (DP 2000) should be deleted. "
            f"P2 field: {[(s.card_id, s.dp) for s in snap.p2_field]}")
        # Should be in opponent's trash
        assert "BT1-010" in snap.p2_trash, (
            f"Deleted Agumon should be in P2 trash. P2 trash: {snap.p2_trash}")

    def test_on_play_token_triggers_your_turn_effect(self, debug_runner):
        """When Jesmon is played and creates a token, the token entering the
        field counts as 'another Digimon played' and triggers the Your Turn
        OPT effect. This is correct behavior per DCGO -- Jesmon's own play
        does not trigger (it checks played_permanent != owner_perm), but the
        token it creates IS a different permanent."""
        runner = debug_runner(initial_memory=15)
        # Place opponent Digimon with DP <= 8000
        runner.place_on_field(2, ["BT1-010"])  # Agumon, DP 2000

        runner.inject_card(1, "BT20-017", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Jesmon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # The token play should trigger the OPT effect, deleting the opponent's
        # low-DP Digimon
        opp_agumon = next(
            (s for s in snap.p2_field if s.card_id == "BT1-010"), None)
        assert opp_agumon is None, (
            "Opponent's Agumon should be deleted -- token play triggers OPT effect")

    def test_your_turn_trigger_skips_high_dp(self, debug_runner):
        """Should not be able to delete opponent Digimon with DP > 8000."""
        runner = debug_runner(initial_memory=15)
        runner.place_on_field(1, ["BT20-017"])
        # Opponent has only high-DP Digimon
        runner.place_on_field(2, ["BT1-025"])  # WarGreymon, DP 11000

        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's WarGreymon should NOT be deleted (DP > 8000)
        opp_wg = next(
            (s for s in snap.p2_field if s.card_id == "BT1-025"), None)
        assert opp_wg is not None, (
            "WarGreymon (DP 11000) should NOT be deleted -- exceeds 8000 DP threshold")

    def test_your_turn_trigger_once_per_turn(self, debug_runner):
        """The trigger should only fire once per turn."""
        runner = debug_runner(initial_memory=15)
        runner.place_on_field(1, ["BT20-017"])
        # Two opponent Digimon with low DP
        runner.place_on_field(2, ["BT1-010"])  # Agumon #1
        runner.place_on_field(2, ["ST1-03"])   # Agumon #2

        # Play first Digimon to trigger
        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        play1 = runner.find_action("Play Agumon")
        assert play1 is not None
        runner.execute(play1)
        runner.auto_resolve()

        snap1 = runner.snapshot()
        # One opponent Digimon should be deleted
        opp_count = len(snap1.p2_field)

        # Play second Digimon -- OPT should NOT fire again
        runner.inject_card(1, "BT1-010", "hand")
        play2 = runner.find_action("Play Agumon")
        if play2 is not None:
            runner.execute(play2)
            runner.auto_resolve()

        snap2 = runner.snapshot()
        # Should NOT have deleted another opponent Digimon (OPT used)
        # We may have 1 less from the first trigger, but not 2 less from original
        assert len(snap2.p2_field) >= opp_count - 0, (
            f"OPT should prevent second trigger. "
            f"After 1st play: {opp_count} opp field, "
            f"after 2nd play: {len(snap2.p2_field)} opp field")

    def test_your_turn_trigger_does_not_fire_on_opponent_turn(self, debug_runner):
        """The trigger should not fire on opponent's turn."""
        runner = debug_runner(initial_memory=15)
        runner.place_on_field(1, ["BT20-017"])
        runner.place_on_field(1, ["BT1-010"])  # Opponent target

        # Switch to player 2's turn
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        runner.inject_card(2, "ST1-03", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        if play is not None:
            runner.execute(play)
            runner.auto_resolve()

        snap = runner.snapshot()
        # Jesmon's trigger should NOT fire on opponent's turn
        # All of P1's Digimon should still be present
        assert len(snap.p1_field) >= 2, (
            "Jesmon's Your Turn trigger should not fire on opponent's turn")

    # ── C3b: Then, may attack (MAY_ATTACK not FORCE_ATTACK) ──────────

    def test_then_may_attack_uses_may_attack_modifier(self, debug_runner):
        """After delete, the selected Digimon should get MAY_ATTACK modifier,
        not FORCE_ATTACK."""
        from digimon_gym.engine.interfaces.modifiers import ModifierType

        runner = debug_runner(initial_memory=15)
        runner.place_on_field(1, ["BT20-017"])
        runner.place_on_field(2, ["BT1-010"])  # target for delete

        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        assert play is not None
        runner.execute(play)

        # After the play, there should be a selection for delete target
        # Auto-resolve the delete selection
        if runner.game.current_phase == GamePhase.SelectTarget:
            legal = runner.action_mask()
            non_decline = [a for a in legal if a != 62]
            if non_decline:
                runner.execute(non_decline[0])

        # Now should be in selection for "which Digimon may attack"
        if runner.game.current_phase == GamePhase.SelectTarget:
            ps = runner.game.pending_selection
            assert ps is not None, "Should have a pending selection for attack"
            # Select a Digimon for attack
            legal = runner.action_mask()
            non_decline = [a for a in legal if a != 62]
            if non_decline:
                runner.execute(non_decline[0])

        runner.auto_resolve()

        # Check that the selected Digimon has MAY_ATTACK, not FORCE_ATTACK
        for perm in runner.game.player1.battle_area:
            if runner.game.modifiers.has_modifier(perm, ModifierType.FORCE_ATTACK):
                pytest.fail(
                    f"{perm.top_card.c_entity_base.card_name_eng} has FORCE_ATTACK "
                    f"but should use MAY_ATTACK for 'may attack' effect")

    def test_then_attack_is_optional(self, debug_runner):
        """The 'may attack' part should be optional -- player can decline."""
        runner = debug_runner(initial_memory=15)
        runner.place_on_field(1, ["BT20-017"])
        runner.place_on_field(2, ["BT1-010"])  # target for delete

        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        assert play is not None
        runner.execute(play)

        # Auto-resolve delete selection
        if runner.game.current_phase == GamePhase.SelectTarget:
            legal = runner.action_mask()
            non_decline = [a for a in legal if a != 62]
            if non_decline:
                runner.execute(non_decline[0])

        # Now for attack selection, decline
        if runner.game.current_phase == GamePhase.SelectTarget:
            decline = runner.find_action("Decline")
            if decline is not None:
                runner.execute(decline)

        runner.auto_resolve()

        # Game should return to Main phase without error
        snap = runner.snapshot()
        assert snap.phase == "Main", (
            f"After declining attack, should return to Main. Got: {snap.phase}")

    def test_no_delete_targets_skips_attack(self, debug_runner):
        """If no valid delete targets, the 'Then' attack should not fire."""
        runner = debug_runner(initial_memory=15)
        runner.place_on_field(1, ["BT20-017"])
        # Opponent has only high-DP Digimon (no valid delete targets)
        runner.place_on_field(2, ["BT1-025"])  # WarGreymon, DP 11000

        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should return to Main without any attack selection
        assert snap.phase == "Main", (
            f"With no delete targets, should skip attack and return to Main. "
            f"Got: {snap.phase}")

        # No Digimon should have MAY_ATTACK
        from digimon_gym.engine.interfaces.modifiers import ModifierType
        for perm in runner.game.player1.battle_area:
            assert not runner.game.modifiers.has_modifier(perm, ModifierType.MAY_ATTACK), (
                "No Digimon should have MAY_ATTACK when delete was skipped (Then semantics)")

    def test_delete_is_not_optional(self, debug_runner):
        """Delete selection should be mandatory (not optional) when targets exist."""
        runner = debug_runner(initial_memory=15)
        runner.place_on_field(1, ["BT20-017"])
        runner.place_on_field(2, ["BT1-010"])  # DP 2000 target

        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        assert play is not None
        runner.execute(play)

        # The delete selection should be mandatory (no Decline option)
        if runner.game.current_phase == GamePhase.SelectTarget:
            ps = runner.game.pending_selection
            assert ps is not None
            assert not ps.is_optional, (
                "Delete selection should be mandatory (is_optional=False)")
