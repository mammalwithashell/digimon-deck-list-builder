"""Behavioral tests for combat mechanics.

Tests cover:
- Attacking player's security decrements security count
- Attacker suspends after attacking
- Attacking a suspended Digimon with higher DP deletes it
- Cannot attack unsuspended Digimon by default
- Summoning sickness prevents attack on the turn a Digimon was played
- No summoning sickness when turn_played=-1
- Freshly played Digimon has summoning sickness
"""

import pytest


class TestSecurityAttack:
    """Tests for attacking the opponent's security stack."""

    def test_attack_security_decrements_count(self, debug_runner):
        """Attacking P2 directly should reduce their security by 1."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        # Place an attacker on P1's field with no summoning sickness
        runner.place_on_field(1, ["ST1-05"], turn_played=-1)  # Birdramon Lv.4, 5000 DP

        before = runner.snapshot()
        p2_sec_before = before.p2_security_count
        assert p2_sec_before > 0, "P2 should have security cards to attack"

        action = runner.find_action("Attack player with Birdramon")
        assert action is not None, (
            "Should find 'Attack player' action for Birdramon"
        )

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert after.p2_security_count < p2_sec_before, (
            f"P2 security should decrease after attack. "
            f"Before: {p2_sec_before}, After: {after.p2_security_count}"
        )

    def test_attacker_suspends_after_attack(self, debug_runner):
        """After attacking, the attacker should be suspended."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-05"], turn_played=-1)  # Birdramon

        action = runner.find_action("Attack player with Birdramon")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        birdramon_slots = [s for s in after.p1_field if s.card_id == "ST1-05"]
        assert len(birdramon_slots) >= 1, "Birdramon should still be on field"
        assert birdramon_slots[0].is_suspended, (
            "Birdramon should be suspended after attacking"
        )


class TestDigimonBattle:
    """Tests for attacking opponent Digimon."""

    def test_attack_suspended_digimon_wins_by_dp(self, debug_runner):
        """Attacking a lower-DP suspended Digimon should delete it."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        # P1: Strong attacker (12000 DP, no summoning sickness)
        runner.place_on_field(1, ["ST1-10"], turn_played=-1)  # Phoenixmon Lv.6, 12000 DP

        # P2: Weak defender, suspended so it's a valid attack target
        runner.place_on_field(2, ["ST1-02"], is_suspended=True, turn_played=-1)

        before = runner.snapshot()
        p2_field_before = len(before.p2_field)

        action = runner.find_action("Attack Biyomon with Phoenixmon")
        assert action is not None, "Should find action to attack suspended Biyomon"

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert len(after.p2_field) < p2_field_before, (
            f"P2 should have fewer Digimon after battle loss. "
            f"Before: {p2_field_before}, After: {len(after.p2_field)}"
        )

    def test_cannot_attack_unsuspended_digimon_normally(self, debug_runner):
        """By default, you cannot attack an unsuspended Digimon."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-10"], turn_played=-1)  # Phoenixmon
        runner.place_on_field(2, ["ST1-02"], is_suspended=False, turn_played=-1)

        action = runner.find_action("Attack Biyomon with Phoenixmon")
        assert action is None, (
            "Should not find action to attack unsuspended Biyomon "
            "(only suspended Digimon are valid attack targets)"
        )


class TestSummoningSickness:
    """Tests for summoning sickness mechanic."""

    def test_summoning_sickness_prevents_attack(self, debug_runner):
        """A Digimon placed this turn (turn_played == current turn) cannot attack."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        current_turn = runner.snapshot().turn
        runner.place_on_field(1, ["ST1-05"], turn_played=current_turn)  # Birdramon

        attack_actions = runner.find_actions("Attack")
        birdramon_attacks = {
            k: v for k, v in attack_actions.items()
            if "Birdramon" in v
        }
        assert len(birdramon_attacks) == 0, (
            f"Digimon with summoning sickness should not have attack actions, "
            f"but found: {birdramon_attacks}"
        )

    def test_no_sickness_with_past_turn_played(self, debug_runner):
        """A Digimon with turn_played=-1 should be able to attack."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-05"], turn_played=-1)  # Birdramon

        action = runner.find_action("Attack player with Birdramon")
        assert action is not None, (
            "Digimon with turn_played=-1 should be able to attack"
        )

    def test_playing_digimon_gives_sickness(self, debug_runner):
        """A Digimon played from hand during this turn should have summoning sickness."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        runner.inject_card(1, "ST1-05", "hand")  # Birdramon, cost 4

        action = runner.find_action("Play Birdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Only check if P1 is still active in Main phase (memory didn't cross zero)
        if snap.active_player == 1 and snap.phase == "Main":
            attack_action = runner.find_action("Attack player with Birdramon")
            assert attack_action is None, (
                "Freshly played Digimon should not be able to attack "
                "(summoning sickness)"
            )
