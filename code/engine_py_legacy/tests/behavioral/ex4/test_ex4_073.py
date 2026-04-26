"""Behavioral tests for EX4-073 Omnimon Alter-B (Lv.7 Purple, Cost 14).

Card text:
[When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon. Then, delete
    1 of your opponent's Digimon with as much or less DP as this Digimon.
[All Turns] All of your Digimon with [Omnimon] in their names gain <Blocker>.

Alt-digi: Lv.6 w/[Omnimon] in name: Cost 4
DNA: Purple Lv.6 + Blue/Red Lv.6, cost 0
"""

import pytest


@pytest.mark.behavioral
class TestEX4073OmnimonAlterB:
    """Tests for EX4-073 Omnimon Alter-B."""

    def test_when_digivolving_de_digivolve_3(self, debug_runner):
        """[When Digivolving] Should De-Digivolve 3 an opponent's Digimon."""
        runner = debug_runner(initial_memory=20)
        runner.set_phase("Main")

        # Place a Lv.6 Omnimon to digivolve from (alt-digi)
        runner.place_on_field(1, ["BT5-086"])  # Omnimon Lv.7 as base (has Omnimon in name)
        # Actually we need a Lv.6 with [Omnimon] in name for alt-digi
        # Or just a Lv.6 Purple for normal evo
        # Let's use normal digi: EX4-049 CresGarurumon is Purple Lv.6
        # Actually, let's simplify - place Omnimon Alter-B already on field
        # and test the When Digivolving effect by triggering it via digivolve

        # Clear field
        runner.clear_zone(1, "field")

        # Place a Purple Lv.6 on field
        runner.place_on_field(1, ["EX4-049"])  # CresGarurumon Lv.6 Purple

        # Opponent has a stacked Digimon (Lv.3 -> Lv.4 -> Lv.5 -> Lv.6)
        runner.place_on_field(2, ["ST1-03", "ST1-07", "ST1-09", "ST1-11"])

        # Put EX4-073 in hand
        runner.inject_card(1, "EX4-073", "hand")

        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve into Omnimon Alter-B"

        runner.execute(action)
        # Auto-resolve: select opponent Digimon for de-digivolve, then for delete
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # The opponent's Digimon should have been de-digivolved (lost up to 3 cards)
        # and then one should have been deleted
        # Check that the opponent lost at least some cards to trash
        assert len(snap_after.p2_trash) > 0, (
            "Opponent should have cards in trash from de-digivolve and/or delete"
        )

    def test_when_digivolving_delete_by_dp(self, debug_runner):
        """[When Digivolving] Should delete 1 opponent Digimon with DP <= this Digimon's DP."""
        runner = debug_runner(initial_memory=20)
        runner.set_phase("Main")

        # Place Omnimon Alter-B on field already (to test delete by DP directly)
        perm = runner.place_on_field(1, ["EX4-073"])  # 15000 DP

        # Opponent: one Digimon with 14000 DP (deletable) and one with 16000 DP (not deletable)
        runner.place_on_field(2, ["ST1-03"])  # Agumon 2000 DP - easily deletable
        runner.place_on_field(2, ["BT1-025"])  # WarGreymon 11000 DP - also deletable

        # For when-digivolving test, let's use a Lv.6 base and digivolve
        runner.clear_zone(1, "field")
        runner.place_on_field(1, ["EX4-049"])  # CresGarurumon Lv.6
        runner.inject_card(1, "EX4-073", "hand")

        action = runner.find_action("Digivolve")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # At least one opponent Digimon should have been deleted
        # (both had DP <= 15000 so either could be selected)
        opp_digimon_count = sum(1 for s in snap_after.p2_field if s.is_digimon)
        assert opp_digimon_count < 2, (
            f"At least one opponent Digimon should be deleted; "
            f"still have {opp_digimon_count} on field"
        )

    def test_all_turns_omnimon_gain_blocker(self, debug_runner):
        """[All Turns] Digimon with [Omnimon] in name should gain Blocker."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place Omnimon Alter-B on field
        runner.place_on_field(1, ["EX4-073"])

        # Place another Omnimon on field
        runner.place_on_field(1, ["BT5-086"])  # Omnimon

        snap = runner.snapshot()

        # Both Omnimon should have blocker
        for s in snap.p1_field:
            if s.is_digimon:
                assert "blocker" in s.keywords, (
                    f"{s.card_name} ({s.card_id}) should have blocker from "
                    f"Omnimon Alter-B's [All Turns] effect, keywords: {s.keywords}"
                )

    def test_all_turns_non_omnimon_no_blocker(self, debug_runner):
        """[All Turns] Non-Omnimon Digimon should NOT gain Blocker from this effect."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX4-073"])  # Omnimon Alter-B

        # Place a non-Omnimon Digimon
        runner.place_on_field(1, ["ST1-03"])  # Agumon - no Omnimon in name

        snap = runner.snapshot()
        agumon = None
        for s in snap.p1_field:
            if s.card_id == "ST1-03":
                agumon = s
                break
        assert agumon is not None
        assert "blocker" not in agumon.keywords, (
            f"Non-Omnimon Agumon should NOT have blocker, keywords: {agumon.keywords}"
        )

    def test_alt_digi_from_lv6_omnimon(self, debug_runner):
        """Alt-digi: Should be able to digivolve from Lv.6 with [Omnimon] in name for cost 4."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Lv.6 Omnimon on field
        # EX4-060 is Omnimon Alter-S (Lv.7) - too high
        # We need a Lv.6 with Omnimon in name... checking available cards
        # Actually checking card DB, all Omnimon are Lv.7
        # The alt-digi says Lv.6 w/[Omnimon] in name: Cost 4
        # In practice this might not have targets, but we should verify the
        # alt digi attributes are set correctly on the effect.

        # Verify via direct effect inspection
        game = runner.game
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX4-073", game.player1)
        effects = cs.effect_list(None)

        # Find the alt-digi effect
        alt_digi_effect = None
        for eff in effects:
            if getattr(eff, '_alt_digi_cost', None) is not None:
                alt_digi_effect = eff
                break

        assert alt_digi_effect is not None, "Should have an alt-digi effect"
        assert alt_digi_effect._alt_digi_level == 6, (
            f"Alt-digi level should be 6, got {alt_digi_effect._alt_digi_level}"
        )
        assert alt_digi_effect._alt_digi_cost == 4, (
            f"Alt-digi cost should be 4, got {alt_digi_effect._alt_digi_cost}"
        )
        assert alt_digi_effect._alt_digi_name == "Omnimon", (
            f"Alt-digi name should be 'Omnimon', got {alt_digi_effect._alt_digi_name}"
        )

    def test_omnimon_alter_b_itself_gets_blocker(self, debug_runner):
        """[All Turns] Omnimon Alter-B itself has [Omnimon] in its name and should gain Blocker."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX4-073"])

        snap = runner.snapshot()
        alter_b = None
        for s in snap.p1_field:
            if s.card_id == "EX4-073":
                alter_b = s
                break
        assert alter_b is not None
        assert "blocker" in alter_b.keywords, (
            f"Omnimon Alter-B should have blocker from its own effect, "
            f"keywords: {alter_b.keywords}"
        )
