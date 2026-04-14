"""Behavioral tests for BT24-024 Submarimon (Lv.4 Blue/Green Digimon).

Card text:
  Alt-digi: [Armadillomon]/Lv.3 w/[TS] trait: Cost 2
  <Armor Purge>
  [When Attacking] [Once Per Turn] You may play 1 Tamer card with the [TS]
  trait from your hand with the play cost reduced by 2.
"""

import pytest


@pytest.mark.behavioral
class TestBT24024Submarimon:
    """Tests for BT24-024 Submarimon."""

    def test_when_attacking_plays_ts_tamer_from_hand(self, debug_runner):
        """Attacking with Submarimon should allow playing a TS tamer from hand."""
        runner = debug_runner(initial_memory=10)

        # Place Submarimon on field (turn_played=-1 so it can attack)
        runner.place_on_field(1, ["BT24-024"])
        # Inject a TS tamer (BT24-083 Hiroko Sagisaka, cost 3) into hand
        runner.inject_card(1, "BT24-083", "hand")

        runner.set_phase("Main")

        # Declare attack
        attack = runner.find_action("Attack player with Submarimon")
        assert attack is not None, "Should be able to attack with Submarimon"
        runner.execute(attack)

        # The When Attacking trigger should present a selection to play a tamer
        # Auto-resolve picks the tamer to play
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # The TS tamer should now be on field
        tamer_on_field = any(
            s.card_id == "BT24-083" for s in snap.p1_field
        )
        assert tamer_on_field, "TS tamer should be played to the field"

    def test_when_attacking_does_not_play_non_ts_tamer(self, debug_runner):
        """Non-TS tamers in hand should not be playable via this effect."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["BT24-024"])
        # Inject a non-TS tamer (ST1-12 Taichi Yagami) into hand
        runner.inject_card(1, "ST1-12", "hand")

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Submarimon")
        assert attack is not None
        runner.execute(attack)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # The non-TS tamer should still be in hand, not played
        tamer_on_field = any(
            s.card_id == "ST1-12" for s in snap.p1_field
        )
        assert not tamer_on_field, "Non-TS tamer should NOT be played by this effect"

    def test_when_attacking_cost_reduced_by_2(self, debug_runner):
        """Playing the tamer should cost 2 less than normal (3 - 2 = 1)."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT24-024"])
        # BT24-083 costs 3, reduced by 2 = 1 memory
        runner.inject_card(1, "BT24-083", "hand")

        runner.set_phase("Main")
        snap_before = runner.snapshot()

        attack = runner.find_action("Attack player with Submarimon")
        assert attack is not None
        runner.execute(attack)
        results = runner.auto_resolve()

        snap_after = runner.snapshot()
        # Memory should decrease by 1 (cost 3 - 2 reduction = 1)
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 1, f"Should spend 1 memory (3 cost - 2 reduction), spent {memory_spent}"

    def test_when_attacking_once_per_turn(self, debug_runner):
        """The effect should only trigger once per turn."""
        runner = debug_runner(initial_memory=15)

        runner.place_on_field(1, ["BT24-024"])
        runner.inject_card(1, "BT24-083", "hand")
        runner.inject_card(1, "BT24-084", "hand")  # Second TS tamer

        runner.set_phase("Main")

        # First attack — should trigger the effect
        attack = runner.find_action("Attack player with Submarimon")
        assert attack is not None
        runner.execute(attack)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        first_tamer_played = any(
            s.card_id in ("BT24-083", "BT24-084") for s in snap.p1_field
        )
        assert first_tamer_played, "First attack should play a TS tamer"

        # Submarimon is now suspended after attacking — unsuspend it for second attack
        game = runner.game
        player = game.player1
        for perm in player.battle_area:
            if perm.top_card and perm.top_card.c_entity_base and perm.top_card.c_entity_base.card_id == "BT24-024":
                perm.unsuspend()
                break

        # Second attack — should NOT trigger the effect again
        runner.set_phase("Main")
        attack2 = runner.find_action("Attack player with Submarimon")
        if attack2 is not None:
            runner.execute(attack2)
            results2 = runner.auto_resolve()

        snap2 = runner.snapshot()
        tamers_on_field = sum(
            1 for s in snap2.p1_field
            if s.card_id in ("BT24-083", "BT24-084")
        )
        assert tamers_on_field <= 1, "Once Per Turn — only 1 tamer should be played across 2 attacks"
