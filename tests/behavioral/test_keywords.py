"""Behavioral tests for keyword mechanics.

Tests cover:
- Rush enables same-turn attack (bypasses summoning sickness)
- Blocker keyword visible in snapshot (using cards with scripts)
- Granted keywords are detected by has_keyword
- Granted keyword expiry respects turn count

Note: Only cards with Python scripts in digimon_gym/engine/data/scripts/ will
have their keywords detected by has_keyword(). Vanilla starter deck cards
without scripts (e.g., ST1-06 Coredramon) won't report keywords even if
listed in their card text.
"""

import pytest


class TestRushKeyword:
    """Tests for the Rush keyword: allows attacking the turn the Digimon enters play."""

    def test_rush_bypasses_summoning_sickness(self, debug_runner):
        """A Digimon with Rush granted this turn should be able to attack
        despite summoning sickness."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        current_turn = runner.snapshot().turn
        perm = runner.place_on_field(1, ["ST1-05"], turn_played=current_turn)

        # Verify it cannot attack yet (summoning sickness)
        action = runner.find_action("Attack player with Birdramon")
        assert action is None, "Should not attack without Rush"

        # Grant Rush keyword directly
        perm.grant_keyword('_is_rush', -1)

        # Now it should be able to attack
        action = runner.find_action("Attack player with Birdramon")
        assert action is not None, (
            "Digimon with Rush should be able to attack despite summoning sickness"
        )

    def test_rush_digimon_can_attack_suspended_target(self, debug_runner):
        """A Digimon with Rush can attack a suspended opponent Digimon on the same turn."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        current_turn = runner.snapshot().turn
        perm = runner.place_on_field(1, ["ST1-10"], turn_played=current_turn)
        perm.grant_keyword('_is_rush', -1)

        runner.place_on_field(2, ["ST1-02"], is_suspended=True, turn_played=-1)

        action = runner.find_action("Attack Biyomon with Phoenixmon")
        assert action is not None, (
            "Rush Digimon should be able to attack suspended opponent Digimon"
        )

    def test_rush_allows_security_attack(self, debug_runner):
        """A Rush Digimon should be able to attack the player (security) on its first turn."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        current_turn = runner.snapshot().turn
        perm = runner.place_on_field(1, ["ST1-05"], turn_played=current_turn)
        perm.grant_keyword('_is_rush', -1)

        action = runner.find_action("Attack player with Birdramon")
        assert action is not None, (
            "Rush Digimon should be able to attack the player on the same turn"
        )


class TestBlockerKeyword:
    """Tests for the Blocker keyword using cards with working scripts."""

    def test_blocker_keyword_visible_in_snapshot(self, debug_runner):
        """BT10-031 Pulsemon has Blocker with a script that sets _is_blocker.
        It should show 'blocker' in the snapshot keywords."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["BT10-031"], turn_played=-1)

        snap = runner.snapshot()
        pulsemon_slots = [s for s in snap.p1_field if s.card_id == "BT10-031"]
        assert len(pulsemon_slots) >= 1, "Pulsemon should be on field"
        assert "blocker" in pulsemon_slots[0].keywords, (
            f"Pulsemon should have 'blocker' keyword, "
            f"got: {pulsemon_slots[0].keywords}"
        )

    def test_blocker_detected_by_has_keyword(self, debug_runner):
        """Direct has_keyword check should detect Blocker on scripted cards."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT10-031"], turn_played=-1)
        assert perm.has_keyword('_is_blocker'), (
            "BT10-031 Pulsemon should have _is_blocker keyword via script"
        )


class TestGrantedKeywords:
    """Tests for keywords granted programmatically at runtime."""

    def test_grant_keyword_is_detected(self, debug_runner):
        """Granting a keyword to a permanent should make it detectable via has_keyword."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["ST1-05"], turn_played=-1)

        assert not perm.has_keyword('_is_blocker'), (
            "Birdramon should not have Blocker before granting"
        )

        perm.grant_keyword('_is_blocker', -1)

        assert perm.has_keyword('_is_blocker'), (
            "Birdramon should have Blocker after granting"
        )

    def test_granted_rush_expires_by_turn(self, debug_runner):
        """A Rush keyword granted with a turn-based expiry should expire
        after the specified turn."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["ST1-05"], turn_played=-1)
        current_turn = runner.snapshot().turn

        # Grant Rush for this turn only
        perm.grant_keyword('_is_rush', current_turn)

        assert perm.has_keyword('_is_rush'), (
            "Rush should be active during the granted turn"
        )

        # Advance the game's turn count past expiry
        runner.game.turn_count = current_turn + 1

        assert not perm.has_keyword('_is_rush'), (
            "Rush should have expired after the granted turn"
        )

    def test_permanent_grant_does_not_expire(self, debug_runner):
        """A keyword granted with duration=-1 should not expire when turns advance."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["ST1-05"], turn_played=-1)

        perm.grant_keyword('_is_rush', -1)

        runner.game.turn_count += 5

        assert perm.has_keyword('_is_rush'), (
            "Permanently granted Rush should not expire"
        )
