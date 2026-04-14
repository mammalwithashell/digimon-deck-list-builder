"""Behavioral tests for EX9-032 Karakurumon (Lv.5 Yellow/Purple Digimon).

Card text:
  Alt-digi: From Lv.4 [Puppet] for cost 3

  [On Play] [When Digivolving] By deleting 1 of your Tokens or other [Puppet]
  trait Digimon, this Digimon may digivolve into a Digimon card with the
  [Puppet] trait in the hand without paying the cost.

  Inherited:
  [All Turns] [Once Per Turn] When this Digimon would leave the battle area
  other than by your effects, by deleting 1 of your Tokens or other [Puppet]
  trait Digimon, prevent it from leaving.
"""

import pytest


@pytest.mark.behavioral
class TestEX9032OnPlayWhenDigivolving:
    """Tests for the [On Play] [When Digivolving] effect:
    Delete 1 Token/other Puppet Digimon -> digivolve into Puppet from hand free."""

    def test_on_play_delete_puppet_then_digivolve(self, debug_runner):
        """On Play: delete another Puppet Digimon, then digivolve into a Puppet from hand."""
        runner = debug_runner(initial_memory=10)

        # Place a Puppet Digimon to be the delete target
        puppet_target = runner.place_on_field(1, ["BT2-055"])  # ToyAgumon Lv.3 Puppet

        # Inject a Puppet Digimon card into hand for digivolve target
        runner.inject_card(1, "BT2-049", "hand")  # Puppetmon Lv.6 Puppet

        # Play EX9-032 from hand
        runner.inject_card(1, "EX9-032", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        assert play is not None, f"Should be able to play Karakurumon. Actions: {runner.actions()}"
        runner.execute(play)

        # On Play effect: first select the Puppet Digimon to delete
        select_target = runner.find_action("ToyAgumon")
        assert select_target is not None, (
            f"Should be able to select ToyAgumon for deletion. Actions: {runner.actions()}")
        runner.execute(select_target)

        # Then select the hand card to digivolve into
        select_digivolve = runner.find_action("Puppetmon")
        assert select_digivolve is not None, (
            f"Should be able to select Puppetmon for digivolve. Actions: {runner.actions()}")
        runner.execute(select_digivolve)
        runner.auto_resolve()

        snap = runner.snapshot()
        # ToyAgumon should be gone (deleted)
        assert not any(s.card_id == "BT2-055" for s in snap.p1_field), (
            "ToyAgumon should have been deleted as cost")
        # Karakurumon should have digivolved into Puppetmon
        assert any(s.card_id == "BT2-049" for s in snap.p1_field), (
            "Puppetmon should be on field after digivolve from hand")
        # The stack should contain EX9-032 under Puppetmon
        puppetmon_slot = next(s for s in snap.p1_field if s.card_id == "BT2-049")
        assert "EX9-032" in puppetmon_slot.stack_ids, (
            f"EX9-032 should be in the digivolve stack. Stack: {puppetmon_slot.stack_ids}")

    def test_on_play_no_delete_target_skips_effect(self, debug_runner):
        """On Play with no Token/Puppet on field: effect doesn't activate."""
        runner = debug_runner(initial_memory=10)

        # No Puppet Digimon or Tokens on field
        runner.inject_card(1, "EX9-032", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Karakurumon should be on field as-is (no digivolve happened)
        assert any(s.card_id == "EX9-032" for s in snap.p1_field), (
            "Karakurumon should be on the field")

    def test_on_play_cannot_delete_self(self, debug_runner):
        """On Play: cannot select itself as the delete target."""
        runner = debug_runner(initial_memory=10)

        # Only Karakurumon on field (self) - no valid delete target
        runner.inject_card(1, "EX9-032", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Karakurumon should still be on field (effect didn't fire - no valid target)
        assert any(s.card_id == "EX9-032" for s in snap.p1_field), (
            "Karakurumon should remain on field")

    def test_on_play_effect_is_optional(self, debug_runner):
        """On Play: the effect is optional ('may digivolve'), player can decline."""
        runner = debug_runner(initial_memory=10)

        # Place a Puppet Digimon as delete target
        runner.place_on_field(1, ["BT2-055"])  # ToyAgumon Lv.3 Puppet
        runner.inject_card(1, "BT2-049", "hand")  # Puppetmon in hand

        runner.inject_card(1, "EX9-032", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Karakurumon")
        runner.execute(play)

        # The selection phase should have a decline/pass option
        decline = runner.find_action("Decline")
        assert decline is not None, (
            f"Effect should be optional (decline available). Actions: {runner.actions()}")
        runner.execute(decline)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Karakurumon on field, ToyAgumon still on field (didn't sacrifice)
        assert any(s.card_id == "EX9-032" for s in snap.p1_field), (
            "Karakurumon should be on field")
        assert any(s.card_id == "BT2-055" for s in snap.p1_field), (
            "ToyAgumon should still be on field (player declined)")


@pytest.mark.behavioral
class TestEX9032InheritedPreventLeaving:
    """Tests for the inherited effect:
    [All Turns][Once Per Turn] When this Digimon would leave the battle area
    other than by your effects, by deleting 1 of your Tokens or other [Puppet]
    trait Digimon, prevent it from leaving."""

    def test_prevent_deletion_by_opponent_effect(self, debug_runner):
        """Inherited: prevents deletion by opponent effect, deletes a Puppet substitute."""
        runner = debug_runner(initial_memory=5)

        # Place a Digimon with EX9-032 as inherited (stack with Puppet digi on top)
        # Stack: BT2-055 (Lv.3 Puppet) -> EX9-032 (inherited) -> BT2-049 (Lv.6 Puppet top)
        main_perm = runner.place_on_field(1, ["BT2-055", "EX9-032", "BT2-049"])

        # Place a separate Puppet Digimon as sacrifice target
        sacrifice = runner.place_on_field(1, ["BT2-055"])  # ToyAgumon

        # Try to delete the main Digimon by opponent effect
        deleted = runner.game.player1.delete_permanent(main_perm, is_opponent_effect=True)

        # Main Digimon should NOT have been deleted (prevention flag set optimistically)
        assert not deleted, (
            "delete_permanent should return False (deletion prevented)")

        # Resolve the substitute selection - pick the non-decline action
        actions = runner.actions()
        non_decline = [aid for aid, desc in actions.items()
                       if "decline" not in desc.lower() and "pass" not in desc.lower()]
        assert len(non_decline) > 0, (
            f"Should have a non-decline action to select substitute. Actions: {actions}")
        runner.execute(non_decline[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT2-049" for s in snap.p1_field), (
            "Main Digimon (Puppetmon) should still be on field - deletion prevented")
        # ToyAgumon should have been deleted as substitute
        assert not any(
            s.card_id == "BT2-055" for s in snap.p1_field
        ), "ToyAgumon should have been deleted as substitute"

    def test_no_prevention_by_own_effect(self, debug_runner):
        """Inherited: does NOT trigger when deletion is by own effect."""
        runner = debug_runner(initial_memory=5)

        # Stack with EX9-032 inherited
        main_perm = runner.place_on_field(1, ["BT2-055", "EX9-032", "BT2-049"])

        # Place a sacrifice target
        sacrifice = runner.place_on_field(1, ["BT2-055"])  # ToyAgumon

        # Delete by own effect (is_opponent_effect=False, default)
        deleted = runner.game.player1.delete_permanent(main_perm, is_opponent_effect=False)
        runner.auto_resolve()

        assert deleted, "Digimon SHOULD be deleted when removed by own effect"

        snap = runner.snapshot()
        # Main Digimon should be gone
        assert not any(s.card_id == "BT2-049" for s in snap.p1_field), (
            "Puppetmon should be gone from field (own effect deletion)")
        # Sacrifice should still be there (not used)
        assert any(s.card_id == "BT2-055" for s in snap.p1_field), (
            "ToyAgumon should still be on field (wasn't sacrificed)")

    def test_no_prevention_without_substitute(self, debug_runner):
        """Inherited: no prevention when there's no Token/Puppet to sacrifice."""
        runner = debug_runner(initial_memory=5)

        # Stack with EX9-032 inherited
        main_perm = runner.place_on_field(1, ["BT2-055", "EX9-032", "BT2-049"])

        # No other Digimon on field - nothing to sacrifice

        deleted = runner.game.player1.delete_permanent(main_perm, is_opponent_effect=True)
        runner.auto_resolve()

        assert deleted, "Digimon SHOULD be deleted when no substitute available"

        snap = runner.snapshot()
        assert not any(s.card_id == "BT2-049" for s in snap.p1_field), (
            "Puppetmon should be gone from field (no substitute)")

    def test_once_per_turn_limit(self, debug_runner):
        """Inherited: once per turn - second deletion in same turn is not prevented."""
        runner = debug_runner(initial_memory=5)

        # Stack with EX9-032 inherited
        main_perm = runner.place_on_field(1, ["BT2-055", "EX9-032", "BT2-049"])

        # Place two sacrifice targets
        sac1 = runner.place_on_field(1, ["BT2-055"])  # ToyAgumon 1
        sac2 = runner.place_on_field(1, ["BT2-055"])  # ToyAgumon 2

        # First deletion attempt - should be prevented
        deleted1 = runner.game.player1.delete_permanent(main_perm, is_opponent_effect=True)
        assert not deleted1, "First deletion should be prevented"
        # Select a sacrifice target (pick the non-decline action)
        actions = runner.actions()
        non_decline = [aid for aid, desc in actions.items()
                       if "decline" not in desc.lower() and "pass" not in desc.lower()]
        if non_decline:
            runner.execute(non_decline[0])
        runner.auto_resolve()

        # Second deletion attempt in same turn - should NOT be prevented (OPT used)
        deleted2 = runner.game.player1.delete_permanent(main_perm, is_opponent_effect=True)
        runner.auto_resolve()

        assert deleted2, "Second deletion should NOT be prevented (once per turn exhausted)"

    def test_inherited_checks_permanent_match(self, debug_runner):
        """Inherited: only triggers for the Digimon it's inherited under, not others."""
        runner = debug_runner(initial_memory=5)

        # Stack with EX9-032 inherited - this is the protected Digimon
        protected = runner.place_on_field(1, ["BT2-055", "EX9-032", "BT2-049"])

        # Place a sacrifice target
        sacrifice = runner.place_on_field(1, ["BT2-055"])  # ToyAgumon

        # Place another Puppet Digimon (NOT protected by EX9-032)
        unprotected = runner.place_on_field(1, ["BT1-038"])  # Monzaemon Lv.5 Puppet

        # Delete the unprotected one - should NOT trigger EX9-032 inherited
        deleted = runner.game.player1.delete_permanent(unprotected, is_opponent_effect=True)
        runner.auto_resolve()

        assert deleted, "Unprotected Digimon SHOULD be deleted (EX9-032 only protects its own stack)"

        snap = runner.snapshot()
        assert not any(s.card_id == "BT1-038" for s in snap.p1_field), (
            "Monzaemon should be gone from field")
        # Sacrifice target should still be there (was not used for unprotected)
        assert any(s.card_id == "BT2-055" for s in snap.p1_field), (
            "ToyAgumon should still be on field (sacrifice not used)")

    def test_inherited_optional_can_decline(self, debug_runner):
        """Inherited: the inner selection is optional (canNoSelect: true in C#)."""
        runner = debug_runner(initial_memory=5)

        # Stack with EX9-032 inherited
        main_perm = runner.place_on_field(1, ["BT2-055", "EX9-032", "BT2-049"])

        # Place a sacrifice target
        sacrifice = runner.place_on_field(1, ["BT2-055"])  # ToyAgumon

        # Delete by opponent effect
        deleted = runner.game.player1.delete_permanent(main_perm, is_opponent_effect=True)

        # Check if there's a selection phase with a pass/skip option
        if runner.game.current_phase.name.startswith("Select"):
            actions = runner.actions()
            has_pass = any("pass" in desc.lower() or "decline" in desc.lower()
                          for desc in actions.values())
            # The inner selection should be optional (canNoSelect: true)
            assert has_pass, (
                f"Inner selection should have a pass/decline option. Actions: {actions}")
        # Clean up
        runner.auto_resolve()
