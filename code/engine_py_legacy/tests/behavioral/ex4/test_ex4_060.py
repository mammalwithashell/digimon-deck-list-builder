"""Behavioral tests for EX4-060 Omnimon Alter-S.

Card text:
[When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less,
and return 1 of your opponent's level 6 or higher Digimon to the bottom of
its owner's deck.

[All Turns] When this Digimon would leave the battle area other than by one
of your effects, play 1 [BlitzGreymon] and 1 [CresGarurumon] from this
Digimon's digivolution cards without paying the costs. Then, place this
Digimon at the bottom of your security stack face down.
"""

import pytest

# Filler deck for runner setup (50 cards)
FILLER = ["ST1-03"] * 4 + ["ST1-07"] * 46


@pytest.mark.behavioral
class TestEX4060WhenDigivolving:
    """Tests for the [When Digivolving] effect of EX4-060."""

    def _trigger_digivolve(self, runner, perm):
        """Trigger the WhenDigivolving effect on a permanent."""
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(
            EffectTiming.WhenDigivolving,
            {"digivolved_permanent": perm},
        )

    def test_delete_dp_8000_or_less(self, debug_runner):
        """Should delete 1 opponent Digimon with DP <= 8000."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        perm = runner.place_on_field(1, ["EX4-060"])
        # Opponent Digimon: Agumon (2000 DP, Lv.3 — deletable)
        runner.place_on_field(2, ["ST1-03"])

        self._trigger_digivolve(runner, perm)

        # The effect enters SelectTarget phase. Use auto_resolve to
        # pick the first legal action (the only target).
        runner.auto_resolve()

        snap_after = runner.snapshot()
        opp_digimon = [s for s in snap_after.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"Opponent Agumon (2000 DP) should have been deleted, "
            f"but field has: {[s.card_name for s in snap_after.p2_field]}"
        )

    def test_return_level_6_plus_to_deck_bottom(self, debug_runner):
        """Should return 1 opponent Lv.6+ Digimon to the bottom of its owner's deck."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        perm = runner.place_on_field(1, ["EX4-060"])
        # Opponent: WarGreymon (Lv.6, 12000 DP — NOT deletable, but returnable)
        runner.place_on_field(2, ["ST1-11"])

        game = runner.game
        p2_lib_before = len(game.player2.library_cards)

        self._trigger_digivolve(runner, perm)

        # No DP<=8000 delete target, so the effect skips to return step.
        # auto_resolve picks the target for the return selection.
        runner.auto_resolve()

        snap_after = runner.snapshot()
        opp_digimon = [s for s in snap_after.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"Opponent WarGreymon should be returned to deck, "
            f"but field has: {[s.card_name for s in snap_after.p2_field]}"
        )
        assert snap_after.p2_library_size > p2_lib_before, (
            "Opponent library should have grown by the returned card"
        )

    def test_delete_and_return_both_targets(self, debug_runner):
        """Should delete DP<=8000 AND return Lv.6+ when both targets exist."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        perm = runner.place_on_field(1, ["EX4-060"])
        # Two opponent Digimon:
        runner.place_on_field(2, ["ST1-03"])   # Agumon: 2000 DP (deletable)
        runner.place_on_field(2, ["ST1-11"])   # WarGreymon: Lv.6, 12000 DP (returnable)

        self._trigger_digivolve(runner, perm)

        # auto_resolve handles both selection phases (delete, then return)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        opp_digimon = [s for s in snap_after.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"Both opponent Digimon should be removed, "
            f"but field has: {[s.card_name for s in snap_after.p2_field]}"
        )

    def test_no_targets_does_nothing(self, debug_runner):
        """When no valid targets exist, the effect does nothing."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        perm = runner.place_on_field(1, ["EX4-060"])
        # No opponent Digimon on field

        self._trigger_digivolve(runner, perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_name == "Omnimon Alter-S" for s in snap.p1_field), (
            "Omnimon Alter-S should still be on field"
        )


@pytest.mark.behavioral
class TestEX4060WhenRemoveField:
    """Tests for the [All Turns] WhenRemoveField effect of EX4-060."""

    def _build_omnimon_stack(self, runner, player_id=1):
        """Build an Omnimon Alter-S permanent with BlitzGreymon and CresGarurumon
        in its digivolution cards."""
        perm = runner.place_on_field(
            player_id,
            ["ST1-03", "EX4-051", "EX4-049", "EX4-060"],
        )
        return perm

    def test_play_blitz_and_cres_on_opponent_delete(self, debug_runner):
        """When opponent deletes Omnimon Alter-S, play BlitzGreymon and
        CresGarurumon from evo cards and place Omnimon at bottom of security."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = self._build_omnimon_stack(runner, player_id=1)

        game = runner.game
        p1 = game.player1
        sec_before = len(p1.security_cards)

        # Simulate opponent deleting this Digimon
        p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()

        # BlitzGreymon should be on our field
        blitz_on_field = [s for s in snap.p1_field if s.card_name == "BlitzGreymon"]
        assert len(blitz_on_field) >= 1, (
            f"BlitzGreymon should be played on field. "
            f"Field: {[s.card_name for s in snap.p1_field]}"
        )

        # CresGarurumon should be on our field
        cres_on_field = [s for s in snap.p1_field if s.card_name == "CresGarurumon"]
        assert len(cres_on_field) >= 1, (
            f"CresGarurumon should be played on field. "
            f"Field: {[s.card_name for s in snap.p1_field]}"
        )

        # Security should grow by 1 (Omnimon placed at bottom)
        sec_after = len(p1.security_cards)
        assert sec_after == sec_before + 1, (
            f"Security should grow by 1. Before: {sec_before}, After: {sec_after}"
        )

        # The Omnimon card should be in security
        omnimon_in_security = any(
            cs.c_entity_base and cs.c_entity_base.card_id == "EX4-060"
            for cs in p1.security_cards
        )
        assert omnimon_in_security, (
            "EX4-060 (Omnimon Alter-S) should be in security stack"
        )

        # Omnimon should NOT be on the field
        omnimon_on_field = [s for s in snap.p1_field if s.card_id == "EX4-060"]
        assert len(omnimon_on_field) == 0, (
            "Omnimon Alter-S should not be on field after WhenRemoveField"
        )

    def test_does_not_trigger_on_own_effect(self, debug_runner):
        """WhenRemoveField should NOT trigger when removed by own effects."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = self._build_omnimon_stack(runner, player_id=1)

        game = runner.game
        p1 = game.player1
        sec_before = len(p1.security_cards)

        # Delete by own effect (is_opponent_effect=False = own effect)
        p1.delete_permanent(perm, is_opponent_effect=False)
        runner.auto_resolve()

        snap = runner.snapshot()

        # BlitzGreymon and CresGarurumon should NOT be played
        blitz_on_field = [s for s in snap.p1_field if s.card_name == "BlitzGreymon"]
        cres_on_field = [s for s in snap.p1_field if s.card_name == "CresGarurumon"]
        assert len(blitz_on_field) == 0, (
            "BlitzGreymon should NOT be played when removed by own effect"
        )
        assert len(cres_on_field) == 0, (
            "CresGarurumon should NOT be played when removed by own effect"
        )

        # Security should not change
        sec_after = len(p1.security_cards)
        assert sec_after == sec_before, (
            f"Security should not change when removed by own effect. "
            f"Before: {sec_before}, After: {sec_after}"
        )

    def test_no_blitz_or_cres_in_evo(self, debug_runner):
        """If no BlitzGreymon/CresGarurumon in evo, just place self to security."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        # Build stack WITHOUT BlitzGreymon or CresGarurumon
        perm = runner.place_on_field(1, ["ST1-03", "ST1-07", "EX4-060"])

        game = runner.game
        p1 = game.player1
        sec_before = len(p1.security_cards)

        p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        # Omnimon should be in security
        omnimon_in_security = any(
            cs.c_entity_base and cs.c_entity_base.card_id == "EX4-060"
            for cs in p1.security_cards
        )
        assert omnimon_in_security, (
            "EX4-060 should still be placed in security even without "
            "BlitzGreymon/CresGarurumon in evo"
        )

    def test_remaining_evo_cards_go_to_trash(self, debug_runner):
        """Evo cards that are NOT BlitzGreymon/CresGarurumon should end up in trash."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = self._build_omnimon_stack(runner, player_id=1)

        game = runner.game
        p1 = game.player1

        p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        # The Agumon (ST1-03) at bottom of evo stack should be in trash
        agumon_in_trash = any(
            cs.c_entity_base and cs.c_entity_base.card_id == "ST1-03"
            for cs in p1.trash_cards
        )
        assert agumon_in_trash, (
            "Non-BlitzGreymon/CresGarurumon evo cards should be in trash"
        )
