"""Behavioral tests for BT13-111 Gallantmon | Lv.6 | Red | DP:13000 | Cost:13.

Card text:
When you would play this card from your hand, if you have no Digimon in play,
reduce the play cost by 2 for every 5 cards in both players' trash.
<Rush>
[On Play] [When Digivolving] [When Attacking] Delete 1 of your opponent's
Digimon with the 6000 DP or less. If no Digimon was deleted by this effect,
delete 1 of your opponent's Digimon with the 13000 DP or more instead.

C# Reference: BT13_111.cs
- Cost reduction: ChangeCostClass, EffectTiming.None, isChangePayingCost=true.
  Only when card in hand, no Digimon in play. Reduction = 2*(both_trash//5).
- Delete: DeletePeremanentAndProcessAccordingToResult with FailureProcess.
  Fallback triggers when deletion FAILS (prevented), not just when no targets.
- canNoSelect=false for both delete steps (mandatory).
"""

import pytest


@pytest.mark.behavioral
class TestBT13111Gallantmon:
    """Tests for BT13-111 Gallantmon."""

    # ── Cost Reduction ──────────────────────────────────────────────────

    def test_cost_reduction_10_trash(self, debug_runner):
        """Cost reduced by 4 when 10 cards in combined trash (2 * (10//5) = 4)."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)

        # Clear field so no Digimon in play
        runner.clear_zone(1, "field")

        # Put 5 cards in each player's trash (10 total)
        for _ in range(5):
            runner.inject_card(1, "ST1-03", "trash")
        for _ in range(5):
            runner.inject_card(2, "ST1-03", "trash")

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None, "Should find play action for Gallantmon"

        result = runner.execute(action)
        # Cost 13 - 4 reduction = 9 memory spent
        expected_cost = 9
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == expected_cost, (
            f"With 10 trash cards, cost should be 13-4=9, but spent {actual_cost}"
        )

    def test_cost_reduction_15_trash(self, debug_runner):
        """Cost reduced by 6 when 15 cards in combined trash (2 * (15//5) = 6)."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")

        for _ in range(10):
            runner.inject_card(1, "ST1-03", "trash")
        for _ in range(5):
            runner.inject_card(2, "ST1-03", "trash")

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        result = runner.execute(action)
        expected_cost = 7  # 13 - 6
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == expected_cost, (
            f"With 15 trash cards, cost should be 13-6=7, but spent {actual_cost}"
        )

    def test_cost_reduction_zero_with_less_than_5_trash(self, debug_runner):
        """No cost reduction when combined trash < 5."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")
        runner.clear_zone(2, "trash")

        # Put only 4 cards in trash
        for _ in range(4):
            runner.inject_card(1, "ST1-03", "trash")

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        result = runner.execute(action)
        expected_cost = 13  # no reduction
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == expected_cost, (
            f"With 4 trash cards, cost should be full 13, but spent {actual_cost}"
        )

    def test_cost_reduction_blocked_with_digimon_in_play(self, debug_runner):
        """Cost reduction does NOT apply when player has Digimon in play."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)

        # Place a Digimon on field
        runner.place_on_field(1, ["ST1-03"])

        # Put 10 cards in trash
        for _ in range(10):
            runner.inject_card(1, "ST1-03", "trash")

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        result = runner.execute(action)
        expected_cost = 13  # no reduction because Digimon in play
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == expected_cost, (
            f"With Digimon in play, cost should be full 13, but spent {actual_cost}"
        )

    def test_cost_reduction_tamer_does_not_block(self, debug_runner):
        """Tamers in play do NOT block the cost reduction (only Digimon do)."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")

        # Place a Tamer on field (not a Digimon)
        # BT13-099 Spencer Damon is a tamer card
        runner.place_on_field(1, ["BT13-099"])

        for _ in range(10):
            runner.inject_card(1, "ST1-03", "trash")

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        result = runner.execute(action)
        # Tamer doesn't count as Digimon, so reduction applies
        expected_cost = 9  # 13 - 4
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == expected_cost, (
            f"With only Tamer in play (no Digimon), cost should be 9, but spent {actual_cost}"
        )

    def test_cost_reduction_no_leak_from_field(self, debug_runner):
        """BT13-111 on field should NOT reduce play cost of other cards."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)

        # Place Gallantmon on field (it's now a Digimon in play)
        runner.place_on_field(1, ["BT13-111"])

        for _ in range(10):
            runner.inject_card(1, "ST1-03", "trash")

        # Try to play a different card — the field Gallantmon's BeforePayCost
        # should NOT reduce the cost of other cards
        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Agumon")
        if action is not None:
            result = runner.execute(action)
            # ST1-03 Agumon costs 3 to play
            # The field Gallantmon's cost reduction should NOT apply
            actual_cost = result.before.memory - result.after.memory
            assert actual_cost == 3, (
                f"Gallantmon on field should not reduce other cards' cost, "
                f"expected 3 but spent {actual_cost}"
            )

    # ── Rush ────────────────────────────────────────────────────────────

    def test_rush_keyword(self, debug_runner):
        """Gallantmon has Rush keyword."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")

        perm = runner.place_on_field(1, ["BT13-111"], turn_played=runner.game.turn_count)

        snap = runner.snapshot()
        gallantmon = None
        for s in snap.p1_field:
            if s.card_id == "BT13-111":
                gallantmon = s
                break

        assert gallantmon is not None
        assert "rush" in gallantmon.keywords, (
            f"Gallantmon should have Rush keyword, got: {gallantmon.keywords}"
        )

    # ── On Play Delete ──────────────────────────────────────────────────

    def test_on_play_deletes_low_dp_target(self, debug_runner):
        """[On Play] Deletes 1 opponent Digimon with DP <= 6000."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")

        # Place a 5000 DP target on opponent's field
        runner.place_on_field(2, ["ST1-03"])  # Agumon, 2000 DP

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # The opponent's Agumon (2000 DP <= 6000) should be deleted
        opp_digimon = [s for s in snap.p2_field if s.is_digimon and s.card_id == "ST1-03"]
        assert len(opp_digimon) == 0, (
            "Opponent's Agumon (2000 DP) should be deleted by On Play effect"
        )

    def test_on_play_fallback_to_high_dp(self, debug_runner):
        """[On Play] When no DP <= 6000 targets exist, fallback to DP >= 13000."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(2, "field")

        # Place ONLY a high DP target (>= 13000) on opponent field
        # AD1-006 Shoutmon X7 has 13000 DP and no script
        runner.place_on_field(2, ["AD1-006"])

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_target = [s for s in snap.p2_field if s.card_id == "AD1-006"]
        assert len(opp_target) == 0, (
            "When no low DP targets exist, should fallback to delete opponent's "
            "Shoutmon X7 (13000 DP >= 13000)"
        )

    def test_on_play_no_fallback_when_low_dp_deleted(self, debug_runner):
        """[On Play] When low DP target IS deleted, do NOT fallback to high DP."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(2, "field")

        # Place both a low DP and high DP target
        runner.place_on_field(2, ["ST1-03"])   # Agumon, 2000 DP
        # AD1-006 Shoutmon X7 has 13000 DP and no script
        runner.place_on_field(2, ["AD1-006"])

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None

        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Low DP target should be deleted
        opp_agumon = [s for s in snap.p2_field if s.card_id == "ST1-03"]
        assert len(opp_agumon) == 0, "Agumon (2000 DP) should be deleted"

        # High DP target should NOT be deleted (low delete succeeded)
        opp_high = [s for s in snap.p2_field if s.card_id == "AD1-006"]
        assert len(opp_high) == 1, (
            "Opponent's high DP Digimon should survive when low DP target was successfully deleted"
        )

    def test_on_play_no_targets(self, debug_runner):
        """[On Play] When no valid targets exist (neither low nor high), nothing happens."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(2, "field")

        # Place a Digimon with DP between 6001-12999 (neither low nor high target)
        # ST1-07 Greymon has 4000 DP — wait that IS a low target.
        # We need something with DP in 6001-12999 range.
        # Let's just have an empty opponent field.

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None

        # Should execute without error even with no targets
        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert not snap.is_game_over, "Game should not be over"
        # Gallantmon should be on field
        own_gallantmon = [s for s in snap.p1_field if s.card_id == "BT13-111"]
        assert len(own_gallantmon) == 1, "Our Gallantmon should be on field"

    # ── When Digivolving Delete ─────────────────────────────────────────

    def test_when_digivolving_deletes_low_dp(self, debug_runner):
        """[When Digivolving] Deletes 1 opponent Digimon with DP <= 6000."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=10)

        # Place an evolution base for Gallantmon (needs Lv.5 Digimon)
        # Use a generic Lv.5 that Gallantmon can digivolve from
        runner.place_on_field(1, ["ST7-05"])  # Lv.5 MegaGrowlmon

        # Place a low DP target on opponent field
        runner.place_on_field(2, ["ST1-03"])  # Agumon, 2000 DP

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        # Find digivolve action
        action = runner.find_action("Digivolve")
        if action is not None:
            result = runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            opp_agumon = [s for s in snap.p2_field if s.card_id == "ST1-03"]
            assert len(opp_agumon) == 0, (
                "Opponent's Agumon should be deleted by When Digivolving effect"
            )

    # ── When Attacking Delete ───────────────────────────────────────────

    def test_when_attacking_deletes_low_dp(self, debug_runner):
        """[When Attacking] Deletes 1 opponent Digimon with DP <= 6000."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=3)

        # Place Gallantmon on field (unsuspended, no summoning sickness)
        runner.place_on_field(1, ["BT13-111"], turn_played=-1)

        # Place a low DP target on opponent field
        runner.place_on_field(2, ["ST1-03"])  # Agumon, 2000 DP

        runner.set_phase("Main")

        # Find attack action
        action = runner.find_action("Attack")
        if action is not None:
            result = runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            opp_agumon = [s for s in snap.p2_field if s.card_id == "ST1-03"]
            assert len(opp_agumon) == 0, (
                "Opponent's Agumon should be deleted by When Attacking effect"
            )

    def test_when_attacking_fallback_high_dp(self, debug_runner):
        """[When Attacking] Fallback to DP >= 13000 when no low DP targets."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=3)
        runner.clear_zone(2, "field")

        # Place Gallantmon on field
        runner.place_on_field(1, ["BT13-111"], turn_played=-1)

        # Place only a high DP target on opponent field (use a different card
        # to avoid OnAllyAttack cross-fire from opponent's BT13-111 effects)
        # AD1-006 Shoutmon X7 has 13000 DP and no script
        runner.place_on_field(2, ["AD1-006"])

        runner.set_phase("Main")

        action = runner.find_action("Attack")
        if action is not None:
            result = runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            opp_target = [s for s in snap.p2_field if s.card_id == "AD1-006"]
            assert len(opp_target) == 0, (
                "Opponent's Shoutmon X7 (13000 DP) should be deleted by fallback"
            )

    # ── Delete Boundary DP Values ───────────────────────────────────────

    def test_delete_exactly_6000_dp(self, debug_runner):
        """Digimon with exactly 6000 DP qualifies as low DP target (<= 6000)."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(2, "field")

        # BT1-019 DarkTyrannomon has exactly 6000 DP
        runner.place_on_field(2, ["BT1-019"])

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_field_ids = [s.card_id for s in snap.p2_field]
        assert "BT1-019" not in opp_field_ids, (
            "Digimon with exactly 6000 DP should be deleted (DP <= 6000)"
        )

    def test_no_delete_mid_dp(self, debug_runner):
        """Digimon with DP between 6001 and 12999 is NOT a valid target for either category."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(2, "field")

        # AD1-002 Aldamon has 8000 DP (not <= 6000, not >= 13000) and no script
        runner.place_on_field(2, ["AD1-002"])

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_field_ids = [s.card_id for s in snap.p2_field]
        assert "AD1-002" in opp_field_ids, (
            "Digimon with 8000 DP should survive (not <= 6000 and not >= 13000)"
        )

    def test_delete_exactly_13000_dp(self, debug_runner):
        """Digimon with exactly 13000 DP qualifies for fallback target (>= 13000)."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(2, "field")

        # AD1-006 Shoutmon X7 has exactly 13000 DP and no script
        runner.place_on_field(2, ["AD1-006"])

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_target = [s for s in snap.p2_field if s.card_id == "AD1-006"]
        assert len(opp_target) == 0, (
            "Digimon with exactly 13000 DP should be deleted by fallback"
        )

    # ── Cost Reduction Calculation Edge Cases ───────────────────────────

    def test_cost_reduction_counts_both_players_trash(self, debug_runner):
        """Cost reduction counts cards from BOTH players' trashes."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")
        runner.clear_zone(2, "trash")

        # 3 in P1 trash + 2 in P2 trash = 5 total -> reduction of 2
        for _ in range(3):
            runner.inject_card(1, "ST1-03", "trash")
        for _ in range(2):
            runner.inject_card(2, "ST1-03", "trash")

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        result = runner.execute(action)

        expected_cost = 11  # 13 - 2
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == expected_cost, (
            f"With 5 total trash cards (3+2), cost should be 13-2=11, but spent {actual_cost}"
        )

    def test_cost_reduction_floor_division(self, debug_runner):
        """Cost reduction uses floor division (9 cards -> 1 group of 5 -> reduction 2)."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")
        runner.clear_zone(2, "trash")

        # 9 total -> 9//5 = 1 -> 2*1 = 2 reduction
        for _ in range(9):
            runner.inject_card(1, "ST1-03", "trash")

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        result = runner.execute(action)

        expected_cost = 11  # 13 - 2
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == expected_cost, (
            f"With 9 trash cards (9//5=1, 2*1=2), cost should be 11, but spent {actual_cost}"
        )

    def test_cost_reduction_large_trash(self, debug_runner):
        """Cost reduction with very large trash (30 cards -> reduction 12, but cost floors at min)."""
        runner = debug_runner(archetype1="Gallantmon", initial_memory=13)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "trash")
        runner.clear_zone(2, "trash")

        # 30 total -> 30//5 = 6 -> 2*6 = 12 reduction
        # Cost 13 - 12 = 1
        for _ in range(30):
            runner.inject_card(1, "ST1-03", "trash")

        runner.inject_card(1, "BT13-111", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gallantmon")
        assert action is not None
        result = runner.execute(action)

        expected_cost = 1  # 13 - 12
        actual_cost = result.before.memory - result.after.memory
        assert actual_cost == expected_cost, (
            f"With 30 trash cards, cost should be 13-12=1, but spent {actual_cost}"
        )
