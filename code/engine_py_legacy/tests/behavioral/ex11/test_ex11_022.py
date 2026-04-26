"""Behavioral tests for EX11-022 Karakurumon (Lv.5 Yellow Digimon).

Card text:
  <Scapegoat> (When this Digimon would be deleted other than by your effects,
  by deleting 1 of your other Digimon, prevent that deletion.)

  [On Play] [When Digivolving] You may play 1 [Puppet] trait Digimon card with
  4000 DP or less from your hand or trash without paying the cost. At turn end,
  delete the Digimon this effect played.

  --- Inherited ---
  [All Turns] [Once Per Turn] When this Digimon would leave the battle area
  other than by your effects, by deleting 1 of your Tokens or other [Puppet]
  trait Digimon, it doesn't leave.

  Alt-digi: From Lv.4 [Puppet] (Yellow/Purple) for cost 3
"""

import pytest

# Minimal deck for tests — avoids Puppet cards in hand that interfere with selection
_FILLER_DECK = ["ST1-02"] * 50


@pytest.mark.behavioral
class TestEX11022Scapegoat:
    """Tests for <Scapegoat> keyword."""

    def test_scapegoat_prevents_deletion_by_opponent(self, debug_runner):
        """Scapegoat deletes another friendly Digimon to prevent own deletion."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        karakurumon = runner.place_on_field(1, ["EX11-022"])
        sacrifice = runner.place_on_field(1, ["BT1-009"])  # Any Digimon

        deleted = runner.game.player1.delete_permanent(
            karakurumon, is_opponent_effect=True)

        assert not deleted, "Karakurumon should NOT be deleted (Scapegoat)"

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-022" for s in snap.p1_field), (
            "Karakurumon should still be on the field")
        assert not any(s.card_id == "BT1-009" for s in snap.p1_field), (
            "Sacrifice Digimon should be gone (deleted by Scapegoat)")

    def test_scapegoat_no_other_digimon(self, debug_runner):
        """Without other Digimon, Scapegoat cannot activate; deletion proceeds."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        karakurumon = runner.place_on_field(1, ["EX11-022"])
        # No other Digimon to sacrifice

        deleted = runner.game.player1.delete_permanent(
            karakurumon, is_opponent_effect=True)

        assert deleted, "Karakurumon should be deleted (no Scapegoat target)"

        snap = runner.snapshot()
        assert not any(s.card_id == "EX11-022" for s in snap.p1_field), (
            "Karakurumon should be gone from the field")


@pytest.mark.behavioral
class TestEX11022OnPlayWhenDigivolving:
    """Tests for [On Play] / [When Digivolving] play-from-zone effect."""

    def test_on_play_plays_puppet_from_hand(self, debug_runner):
        """On Play can play a Puppet Digimon with DP<=4000 from hand for free."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        # Clear hand to avoid interference from deck cards
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-020", "hand")  # Hanimon, Puppet, Yellow/Purple
        runner.inject_card(1, "EX11-022", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        assert play is not None, f"Should find Play action. Actions: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-022" for s in snap.p1_field), (
            "Karakurumon should be on the field")
        # Hanimon should also be played from hand
        assert any(s.card_id == "EX11-020" for s in snap.p1_field), (
            "Hanimon should have been played from hand by the On Play effect")

    def test_on_play_plays_puppet_from_trash(self, debug_runner):
        """On Play can play a Puppet Digimon with DP<=4000 from trash for free."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        # Clear hand, inject only Karakurumon
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-020", "trash")  # Hanimon in trash
        runner.inject_card(1, "EX11-022", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        assert play is not None
        runner.execute(play)

        # The selection offers trash card (action 130+) and Decline (62).
        # auto_resolve picks first legal which is Decline, so manually pick
        # the trash card selection action.
        from engine_py_legacy.engine.data.enums import GamePhase
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget phase, got {runner.game.current_phase}")
        # Find the non-decline action (the trash selection)
        legal = runner.action_mask()
        trash_action = [a for a in legal if a != 62]
        assert len(trash_action) > 0, (
            f"Should have a trash card to select. Legal: {legal}, Actions: {runner.actions()}")
        runner.execute(trash_action[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-022" for s in snap.p1_field), (
            "Karakurumon should be on the field")
        assert any(s.card_id == "EX11-020" for s in snap.p1_field), (
            "Hanimon should have been played from trash by the On Play effect")

    def test_on_play_rejects_non_puppet(self, debug_runner):
        """On Play does not allow playing non-Puppet Digimon."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-009", "hand")  # Monodramon, not Puppet
        runner.inject_card(1, "EX11-022", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-022" for s in snap.p1_field), (
            "Karakurumon should be on the field")
        assert not any(s.card_id == "BT1-009" for s in snap.p1_field), (
            "Monodramon should NOT be on the field (not Puppet)")

    def test_on_play_rejects_puppet_over_4000_dp(self, debug_runner):
        """On Play does not allow playing Puppet Digimon with DP > 4000."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT6-084", "hand")  # Sistermon Ciel, Puppet, DP 5000
        runner.inject_card(1, "EX11-022", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-022" for s in snap.p1_field), (
            "Karakurumon should be on the field")
        assert not any(s.card_id == "BT6-084" for s in snap.p1_field), (
            "Sistermon Ciel should NOT be played (DP too high)")

    def test_end_of_turn_deletes_played_digimon(self, debug_runner):
        """The Digimon played by the effect is deleted at end of turn."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-020", "hand")  # Hanimon
        runner.inject_card(1, "EX11-022", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        # Verify Hanimon is on field
        snap = runner.snapshot()
        assert any(s.card_id == "EX11-020" for s in snap.p1_field), (
            "Hanimon should be on the field before turn end")

        # End the turn — pass turn action
        pass_action = runner.find_action("Pass")
        if pass_action is not None:
            runner.execute(pass_action)
            runner.auto_resolve()

        # After turn end, the played Digimon should be deleted
        snap = runner.snapshot()
        assert not any(s.card_id == "EX11-020" for s in snap.p1_field), (
            "Hanimon should be deleted at end of turn")
        # Check it went to trash
        assert "EX11-020" in snap.p1_trash, (
            "Hanimon should be in trash after end-of-turn deletion")


@pytest.mark.behavioral
class TestEX11022Inherited:
    """Tests for inherited prevent-leaving effect."""

    def test_inherited_prevents_deletion_by_opponent(self, debug_runner):
        """Inherited: delete Token/Puppet to prevent this Digimon from leaving."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Stack: bottom=Hanimon (Lv3), top=Karakurumon (Lv5, inherited effect source)
        host_perm = runner.place_on_field(1, ["EX11-020", "EX11-022"])
        # Another Puppet Digimon to sacrifice
        puppet_sacrifice = runner.place_on_field(1, ["EX7-025"])  # ShoeShoemon, Puppet Lv4

        # Try to delete the host by opponent's effect
        deleted = runner.game.player1.delete_permanent(
            host_perm, is_opponent_effect=True)

        assert not deleted, (
            "Host Digimon should NOT be deleted (inherited prevent-leaving)")

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-022" for s in snap.p1_field), (
            "Host with Karakurumon should still be on the field")
        # The sacrifice should have been deleted
        assert not any(s.card_id == "EX7-025" for s in snap.p1_field), (
            "ShoeShoemon should have been deleted as the sacrifice")

    def test_inherited_does_not_trigger_by_own_effect(self, debug_runner):
        """Inherited: does NOT trigger when deletion is by own effects."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Stack EX11-022 as digi-source under AD1-015 (no Scapegoat on top card)
        host_perm = runner.place_on_field(1, ["EX11-022", "AD1-015"])
        puppet_sacrifice = runner.place_on_field(1, ["EX7-025"])

        # Delete by own effect
        deleted = runner.game.player1.delete_permanent(
            host_perm, is_opponent_effect=False)

        assert deleted, (
            "Host Digimon SHOULD be deleted (own effect, inherited doesn't trigger)")

        snap = runner.snapshot()
        assert not any(s.card_id == "AD1-015" for s in snap.p1_field), (
            "Host should be gone from the field")
        # Sacrifice should still be there (wasn't used)
        assert any(s.card_id == "EX7-025" for s in snap.p1_field), (
            "ShoeShoemon should still be on the field (sacrifice not needed)")

    def test_inherited_no_sacrifice_available(self, debug_runner):
        """Inherited: without Token/Puppet sacrifice, cannot prevent leaving."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # EX11-022 as digi-source under AD1-015 (no Scapegoat on top)
        host_perm = runner.place_on_field(1, ["EX11-022", "AD1-015"])
        # No Puppet or Token to sacrifice -- only a non-Puppet Digimon
        runner.place_on_field(1, ["BT1-009"])  # Monodramon, not Puppet

        deleted = runner.game.player1.delete_permanent(
            host_perm, is_opponent_effect=True)

        assert deleted, (
            "Host should be deleted (no valid sacrifice for inherited effect)")

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited: once per turn -- second deletion in same turn is not prevented."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # EX11-022 as digi-source under AD1-015 (no Scapegoat on top)
        host_perm = runner.place_on_field(1, ["EX11-022", "AD1-015"])
        puppet1 = runner.place_on_field(1, ["EX7-025"])  # ShoeShoemon, Puppet
        puppet2 = runner.place_on_field(1, ["EX7-024"])  # Shoemon, Puppet

        # First deletion -- should be prevented
        deleted1 = runner.game.player1.delete_permanent(
            host_perm, is_opponent_effect=True)
        assert not deleted1, "First deletion should be prevented"

        # Host should still be on field
        snap = runner.snapshot()
        assert any(s.card_id == "AD1-015" for s in snap.p1_field), (
            "Host should still be on field after first prevention")

        # Second deletion -- OPT used, should go through
        deleted2 = runner.game.player1.delete_permanent(
            host_perm, is_opponent_effect=True)
        assert deleted2, (
            "Second deletion should proceed (Once Per Turn already used)")

    def test_inherited_can_sacrifice_token(self, debug_runner):
        """Inherited: can sacrifice a Token to prevent leaving."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        host_perm = runner.place_on_field(1, ["EX11-020", "EX11-022"])

        # Create a token on the field using the engine's token API
        runner.game.effect_play_token(runner.game.player1, 'familiar')

        deleted = runner.game.player1.delete_permanent(
            host_perm, is_opponent_effect=True)

        assert not deleted, "Host should NOT be deleted (Token sacrifice)"

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-022" for s in snap.p1_field), (
            "Host should still be on the field")


@pytest.mark.behavioral
class TestEX11022AltDigivolve:
    """Tests for alternate digivolution requirement."""

    def test_alt_digi_from_yellow_puppet_lv4(self, debug_runner):
        """Can alt-digivolve from Yellow Lv.4 Puppet for cost 3."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # EX7-025: ShoeShoemon, Puppet, Yellow, Lv4
        runner.place_on_field(1, ["EX7-025"])
        runner.inject_card(1, "EX11-022", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        assert digi is not None, (
            f"Should find digivolve action. Actions: {runner.actions()}")

    def test_alt_digi_from_purple_puppet_lv4(self, debug_runner):
        """Can alt-digivolve from Purple Lv.4 Puppet for cost 3."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # BT13-081: Porcupamon, Puppet, Purple, Lv4
        runner.place_on_field(1, ["BT13-081"])
        runner.inject_card(1, "EX11-022", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        assert digi is not None, (
            f"Should find digivolve action from Purple Puppet Lv4. "
            f"Actions: {runner.actions()}")

    def test_no_alt_digi_from_non_yellow_purple_puppet_lv4(self, debug_runner):
        """Cannot alt-digivolve from non-Yellow/Purple Lv.4 Puppet."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # BT13-093: Omekamon, Puppet, White, Lv4
        runner.place_on_field(1, ["BT13-093"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-022", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        # Should NOT find a digivolve action (White Puppet Lv4 doesn't match
        # alt-digi Yellow/Purple requirement, and normal digi needs Lv4 Yellow)
        assert digi is None, (
            f"Should NOT find digivolve from White Puppet Lv4. "
            f"Actions: {runner.actions()}")
