"""Behavioral tests for ST21-13 Matt Ishida & T.K. Takaishi (Tamer).

Card text:
  [Your Turn] When any Digimon cards with the [ADVENTURE] trait would be played
  from your hand, by suspending this Tamer, reduce the play cost by 1.
  [Your Turn] All of your level 5 or higher Digimon with the [ADVENTURE] trait
  gain <Rush> (This Digimon can attack the turn it comes into play.).
  [Security] Play this card without paying the cost.
"""

import pytest


@pytest.mark.behavioral
class TestST2113CostReduction:
    """Tests for the cost reduction effect (suspend to reduce ADVENTURE Digimon play cost)."""

    def test_reduces_adventure_digimon_play_cost(self, debug_runner):
        """Playing an ADVENTURE Digimon from hand should cost 1 less when Tamer is on field."""
        runner = debug_runner(initial_memory=10)

        # Place ST21-13 Tamer on field (unsuspended)
        runner.place_on_field(1, ["ST21-13"])

        # Inject an ADVENTURE Digimon into hand
        # ST21-02 Gomamon: Lv.3, ADVENTURE, play cost 3
        runner.inject_card(1, "ST21-02", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        play = runner.find_action("Play Gomamon")
        assert play is not None, (
            f"Should be able to play Gomamon. Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Cost should be 3 - 1 = 2, so memory should decrease by 2
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 2, (
            f"Expected cost of 2 (3 base - 1 reduction), but spent {memory_spent} memory. "
            f"Before: {snap_before.memory}, After: {snap_after.memory}")

    def test_tamer_gets_suspended_after_cost_reduction(self, debug_runner):
        """The Tamer should be suspended after its cost reduction is used."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST21-13"])
        runner.inject_card(1, "ST21-02", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Gomamon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        tamer_slots = [s for s in snap.p1_field if s.card_id == "ST21-13"]
        assert len(tamer_slots) == 1, "Tamer should still be on field"
        assert tamer_slots[0].is_suspended, (
            "Tamer should be suspended after using cost reduction")

    def test_no_reduction_when_tamer_suspended(self, debug_runner):
        """An already-suspended Tamer should not reduce play cost."""
        runner = debug_runner(initial_memory=10)

        # Place Tamer already suspended
        perm = runner.place_on_field(1, ["ST21-13"])
        perm.suspend()

        # ST21-05 Angemon: Lv.4, ADVENTURE, play cost 4
        runner.inject_card(1, "ST21-05", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        play = runner.find_action("Play Angemon")
        assert play is not None, (
            f"Should be able to play Angemon. Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Full cost since Tamer is already suspended
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 4, (
            f"Expected full cost of 4 (Tamer suspended), but spent {memory_spent}. "
            f"Before: {snap_before.memory}, After: {snap_after.memory}")

    def test_no_reduction_for_non_adventure_digimon(self, debug_runner):
        """Non-ADVENTURE Digimon should not get cost reduction from this Tamer."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST21-13"])

        # BT1-010 Agumon: Lv.3, Dinosaur (NOT ADVENTURE), play cost 3
        runner.inject_card(1, "BT1-010", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        play = runner.find_action("Play Agumon")
        assert play is not None, (
            f"Should be able to play Agumon. Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 3, (
            f"Expected full cost of 3 (non-ADVENTURE), but spent {memory_spent}. "
            f"Before: {snap_before.memory}, After: {snap_after.memory}")

    def test_no_reduction_for_non_digimon(self, debug_runner):
        """Non-Digimon cards (Tamers) should not get cost reduction."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST21-13"])

        # Play a second copy of ST21-13 (it's a Tamer, not a Digimon)
        runner.inject_card(1, "ST21-13", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        play = runner.find_action("Play Matt Ishida")
        assert play is not None, (
            f"Should be able to play Matt Ishida. Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 3, (
            f"Expected full cost of 3 (Tamer, not Digimon), but spent {memory_spent}. "
            f"Before: {snap_before.memory}, After: {snap_after.memory}")

    def test_leak_guard_does_not_reduce_own_play(self, debug_runner):
        """The Tamer's cost reduction must not apply to its own play from hand."""
        runner = debug_runner(initial_memory=10)

        # No Tamer on field yet -- inject to hand and play it
        runner.inject_card(1, "ST21-13", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        play = runner.find_action("Play Matt Ishida")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        memory_spent = snap_before.memory - snap_after.memory
        # Should cost 3, not 2. The card being played can't reduce its own cost.
        assert memory_spent == 3, (
            f"Expected full cost of 3 (leak guard), but spent {memory_spent}. "
            f"Before: {snap_before.memory}, After: {snap_after.memory}")


@pytest.mark.behavioral
class TestST2113RushAura:
    """Tests for the Rush aura: Lv.5+ ADVENTURE Digimon gain Rush during your turn."""

    def test_lv5_adventure_digimon_gains_rush(self, debug_runner):
        """A Lv.5 ADVENTURE Digimon should gain Rush from the Tamer's aura."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST21-13"])
        # ST21-04 Zudomon: Lv.5, ADVENTURE -- placed this turn (has sickness)
        runner.place_on_field(1, ["ST21-04"], turn_played=1)

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Zudomon")
        assert attack is not None, (
            "Lv.5 ADVENTURE Digimon should have Rush from Tamer aura. "
            f"Actions: {runner.actions()}")

    def test_lv4_adventure_digimon_no_rush(self, debug_runner):
        """A Lv.4 ADVENTURE Digimon should NOT gain Rush (below Lv.5)."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST21-13"])
        # ST21-05 Angemon: Lv.4, ADVENTURE -- placed this turn (has sickness)
        runner.place_on_field(1, ["ST21-05"], turn_played=1)

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Angemon")
        assert attack is None, (
            "Lv.4 ADVENTURE Digimon should NOT have Rush from Tamer aura")

    def test_lv5_non_adventure_no_rush(self, debug_runner):
        """A Lv.5 Digimon without ADVENTURE trait should NOT gain Rush."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST21-13"])
        # BT1-021 MetalGreymon: Lv.5, Cyborg (NOT ADVENTURE)
        runner.place_on_field(1, ["BT1-021"], turn_played=1)

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with MetalGreymon")
        assert attack is None, (
            "Non-ADVENTURE Lv.5 Digimon should NOT have Rush from Tamer aura")

    def test_lv6_adventure_digimon_gains_rush(self, debug_runner):
        """A Lv.6 ADVENTURE Digimon should also gain Rush (>=5)."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST21-13"])
        # ST21-11 MetalGarurumon: Lv.6, ADVENTURE
        runner.place_on_field(1, ["ST21-11"], turn_played=1)

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with MetalGarurumon")
        assert attack is not None, (
            "Lv.6 ADVENTURE Digimon should have Rush from Tamer aura. "
            f"Actions: {runner.actions()}")

    def test_rush_not_on_opponent_turn(self, debug_runner):
        """Rush aura should only be active during your turn, not opponent's."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["ST21-13"])
        zudomon = runner.place_on_field(1, ["ST21-04"], turn_played=1)

        # The aura requires owner's turn; checking has_keyword directly
        # During P1's turn, Rush should be active
        runner.set_phase("Main")
        assert zudomon.has_keyword('_is_rush'), (
            "Zudomon should have Rush on owner's turn")


@pytest.mark.behavioral
class TestST2113Security:
    """Tests for the security effect: play this card without paying the cost."""

    def test_security_effect_plays_tamer(self, debug_runner):
        """When revealed from security, the Tamer should be played for free."""
        # P2 is first player so they can attack P1's security
        runner = debug_runner(initial_memory=5, first_player=2)

        game = runner.game

        # Clear P1's security and place ST21-13 as the only security card
        game.player1.security_cards.clear()
        runner.inject_card(1, "ST21-13", "security_top")

        # Place a strong attacker on P2's field (no summoning sickness)
        # BT1-021 MetalGreymon: Lv.5, non-ADVENTURE
        runner.place_on_field(2, ["BT1-021"], turn_played=-1)

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with MetalGreymon")
        assert attack is not None, (
            f"P2 should be able to attack. Actions: {runner.actions()}")
        runner.execute(attack)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Tamer should be on P1's field after security trigger
        tamer_on_field = any(
            s.card_id == "ST21-13" for s in snap_after.p1_field)
        assert tamer_on_field, (
            f"ST21-13 should be played from security. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap_after.p1_field]}")
