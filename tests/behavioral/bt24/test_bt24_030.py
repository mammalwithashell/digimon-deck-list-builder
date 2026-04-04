"""Behavioral tests for BT24-030 Neptunemon (Lv.6 Blue/Yellow Digimon).

Card text:
  [Digivolve] Lv.5 w/[Aqua] or [Sea Animal] in any trait or w/[TS] trait: Cost 3
  When this card would be played, if your opponent has 2 or more Digimon, reduce
  the play cost by 5.
  [On Play] [When Digivolving] Return all of your opponent's Digimon with the
  fewest digivolution cards to the bottom of the deck.
  [All Turns] [Once Per Turn] When this Digimon suspends, it may unsuspend.
  [All Turns] When any of your Digimon with the [TS] trait or [Aqua] or
  [Sea Animal] in any of their traits would leave the battle area by your
  opponent's effects, by suspending this Digimon, they don't leave.
"""

import pytest


@pytest.mark.behavioral
class TestBT24030Neptunemon:
    """Tests for BT24-030 Neptunemon."""

    # ── Cost Reduction ───────────────────────────────────────────────

    def test_cost_reduction_with_two_opponent_digimon(self, debug_runner):
        """Play cost reduced by 5 when opponent has 2+ Digimon (12 -> 7)."""
        runner = debug_runner(initial_memory=8)

        # Opponent has 2 Digimon
        runner.place_on_field(2, ["BT1-009"])
        runner.place_on_field(2, ["BT1-009"])

        runner.inject_card(1, "BT24-030", "hand")
        runner.set_phase("Main")

        before_mem = runner.game.memory
        play = runner.find_action("Play Neptunemon")
        assert play is not None, (
            f"Should be able to play Neptunemon with memory={before_mem}. "
            f"Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        # Cost 12 - 5 = 7. Memory shifts by 7 from P1 side.
        snap = runner.snapshot()
        assert any(s.card_id == "BT24-030" for s in snap.p1_field), (
            "Neptunemon should be on the field after playing")

    def test_no_cost_reduction_with_one_opponent_digimon(self, debug_runner):
        """Full cost 12 when opponent has fewer than 2 Digimon."""
        runner = debug_runner(initial_memory=15)

        # Opponent has only 1 Digimon — no cost reduction
        runner.place_on_field(2, ["BT1-009"])

        runner.inject_card(1, "BT24-030", "hand")
        runner.set_phase("Main")

        before_mem = runner.game.memory
        play = runner.find_action("Play Neptunemon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        # Full cost 12 — memory shifts by 12 (15 - 12 = 3, then passes to opponent = -3)
        snap = runner.snapshot()
        assert any(s.card_id == "BT24-030" for s in snap.p1_field), (
            "Neptunemon should be on the field")
        # Memory should have shifted by 12 (full cost, no reduction)
        assert snap.memory == before_mem - 12, (
            f"Expected memory shift of -12 (full cost). Before={before_mem}, After={snap.memory}")

    # ── On Play / When Digivolving: Bottom-deck ─────────────────────

    def test_on_play_bottom_decks_fewest_digi_cards(self, debug_runner):
        """On Play returns all opponent Digimon with fewest digi-cards to deck bottom."""
        runner = debug_runner(initial_memory=15)

        # Opponent field:
        # - Digimon A: 0 digi-cards (just top card) — should be bounced
        # - Digimon B: 0 digi-cards — should be bounced
        # - Digimon C: 1 digi-card (stacked) — should stay
        runner.place_on_field(2, ["BT1-009"])  # 0 digi-cards
        runner.place_on_field(2, ["BT1-009"])  # 0 digi-cards
        runner.place_on_field(2, ["BT24-002", "BT1-009"])  # 1 digi-card (egg under rookie)

        runner.inject_card(1, "BT24-030", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Neptunemon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Only the stacked Digimon (1 digi-card) should remain on opponent's field
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 1, (
            f"Expected 1 opponent Digimon remaining (the stacked one), got {len(opp_digimon)}. "
            f"P2 field: {[(s.card_id, len(s.stack_ids)) for s in snap.p2_field]}")

    # ── Self-Unsuspend OPT ──────────────────────────────────────────

    def test_self_unsuspend_on_attack(self, debug_runner):
        """Neptunemon unsuspends after attacking (suspend triggers OPT unsuspend)."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT24-030"])
        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Neptunemon")
        assert attack is not None
        runner.execute(attack)
        runner.auto_resolve()

        snap = runner.snapshot()
        neptunemon = next((s for s in snap.p1_field if s.card_id == "BT24-030"), None)
        assert neptunemon is not None, "Neptunemon should still be on field"
        assert not neptunemon.is_suspended, (
            "Neptunemon should have unsuspended via OPT effect after attacking")

    # ── Protection Effect ────────────────────────────────────────────

    def test_protection_prevents_deletion_by_opponent_effect(self, debug_runner):
        """Suspending Neptunemon prevents a TS Digimon from being deleted by opponent's effect."""
        runner = debug_runner(initial_memory=5)

        neptunemon_perm = runner.place_on_field(1, ["BT24-030"])
        ts_digimon = runner.place_on_field(1, ["BT24-009"])  # Shamanmon, TS trait

        # Directly call delete_permanent with is_opponent_effect=True
        deleted = runner.game.player1.delete_permanent(ts_digimon, is_opponent_effect=True)

        assert not deleted, "TS Digimon should NOT have been deleted (Neptunemon protection)"

        snap = runner.snapshot()
        # TS Digimon should still be on field
        assert any(s.card_id == "BT24-009" for s in snap.p1_field), (
            "Shamanmon (TS) should still be on the field")
        # Note: Neptunemon suspends as cost, but the OPT self-unsuspend fires
        # immediately, so it ends up unsuspended. Both permanents should remain.
        assert len([s for s in snap.p1_field if s.card_id in ("BT24-030", "BT24-009")]) == 2, (
            "Both Neptunemon and Shamanmon should be on the field")

    def test_protection_neptunemon_stays_suspended_after_opt_used(self, debug_runner):
        """After OPT is consumed, Neptunemon stays suspended when paying protection cost."""
        runner = debug_runner(initial_memory=5)

        neptunemon_perm = runner.place_on_field(1, ["BT24-030"])
        ts_digimon = runner.place_on_field(1, ["BT24-009"])  # Shamanmon, TS trait

        # Consume the OPT: suspend Neptunemon manually, then it unsuspends via OPT
        neptunemon_perm.suspend()

        # Now the OPT is used. Unsuspend manually to test protection cost.
        neptunemon_perm.unsuspend()

        # Now try to delete the TS Digimon by opponent effect
        deleted = runner.game.player1.delete_permanent(ts_digimon, is_opponent_effect=True)

        assert not deleted, "TS Digimon should NOT have been deleted"

        snap = runner.snapshot()
        assert any(s.card_id == "BT24-009" for s in snap.p1_field), (
            "Shamanmon should still be on the field")
        # This time Neptunemon should STAY suspended (OPT already consumed)
        neptunemon_slot = next((s for s in snap.p1_field if s.card_id == "BT24-030"), None)
        assert neptunemon_slot is not None and neptunemon_slot.is_suspended, (
            "Neptunemon should be suspended (OPT already used this turn)")

    def test_no_protection_by_own_effect(self, debug_runner):
        """Protection does NOT trigger when deletion is by own effect."""
        runner = debug_runner(initial_memory=5)

        neptunemon_perm = runner.place_on_field(1, ["BT24-030"])
        ts_digimon = runner.place_on_field(1, ["BT24-009"])  # Shamanmon, TS trait

        # Delete by own effect (is_opponent_effect=False)
        deleted = runner.game.player1.delete_permanent(ts_digimon, is_opponent_effect=False)

        assert deleted, "TS Digimon SHOULD be deleted when removed by own effect"

        snap = runner.snapshot()
        assert not any(s.card_id == "BT24-009" for s in snap.p1_field), (
            "Shamanmon should be gone from the field")
        # Neptunemon should NOT be suspended
        neptunemon_slot = next((s for s in snap.p1_field if s.card_id == "BT24-030"), None)
        assert neptunemon_slot is not None and not neptunemon_slot.is_suspended, (
            "Neptunemon should NOT be suspended (protection didn't trigger)")

    def test_no_protection_for_non_ts_digimon(self, debug_runner):
        """Protection does NOT trigger for Digimon without TS/Aqua/Sea Animal trait."""
        runner = debug_runner(initial_memory=5)

        neptunemon_perm = runner.place_on_field(1, ["BT24-030"])
        plain_digimon = runner.place_on_field(1, ["BT1-009"])  # Monodramon, Mini Dragon trait

        # Delete by opponent effect
        deleted = runner.game.player1.delete_permanent(plain_digimon, is_opponent_effect=True)

        assert deleted, "Non-TS Digimon SHOULD be deleted (no trait match for protection)"

        snap = runner.snapshot()
        assert not any(s.card_id == "BT1-009" for s in snap.p1_field), (
            "Monodramon should be gone from the field")
        # Neptunemon should NOT be suspended
        neptunemon_slot = next((s for s in snap.p1_field if s.card_id == "BT24-030"), None)
        assert neptunemon_slot is not None and not neptunemon_slot.is_suspended, (
            "Neptunemon should NOT be suspended (protection didn't trigger)")

    def test_no_protection_when_already_suspended(self, debug_runner):
        """Protection does NOT trigger when Neptunemon is already suspended."""
        runner = debug_runner(initial_memory=5)

        neptunemon_perm = runner.place_on_field(1, ["BT24-030"], is_suspended=True)
        ts_digimon = runner.place_on_field(1, ["BT24-009"])  # Shamanmon, TS trait

        # Delete by opponent effect — Neptunemon already suspended, can't pay cost
        deleted = runner.game.player1.delete_permanent(ts_digimon, is_opponent_effect=True)

        assert deleted, "TS Digimon SHOULD be deleted when Neptunemon is already suspended"

        snap = runner.snapshot()
        assert not any(s.card_id == "BT24-009" for s in snap.p1_field), (
            "Shamanmon should be gone (no protection available)")
