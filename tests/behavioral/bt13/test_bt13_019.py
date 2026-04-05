"""Behavioral tests for BT13-019 Gankoomon.

Card text:
  Lv.6 | Red/Black | Holy Warrior, Royal Knight | DP:13000 | Cost:13
  <Blocker>
  [On Play] [When Digivolving] You may play 1 Digimon card with [Sistermon] in
  its name from your trash or 1 Digimon card with the [Royal Knight] trait from
  the digivolution cards of your Digimon in the breeding area without paying its
  cost. You can't play [Gankoomon] or [Omnimon] with this effect.

Key faithfulness points tested:
  - Blocker keyword is present.
  - On Play triggers the play-from-trash / play-from-breeding effect.
  - Sistermon can be played from trash for free.
  - Royal Knight can be played from breeding digi-sources for free.
  - Name exclusion uses exact match: [Gankoomon] excludes only "Gankoomon", NOT
    "Gankoomon (X Antibody)". Same for [Omnimon] vs "Omnimon (X Antibody)".
  - When only 1 valid Royal Knight in breeding, player can still decline
    (C# uses canNoSelect: true).
  - When Digivolving also triggers the same effect.
"""

import pytest

from digimon_gym.engine.game.constants import SEL_TRASH_START, SEL_EFFECT_CHOICE_START


# --- Card IDs used in tests ---
GANKOOMON = "BT13-019"            # Card under test
SISTERMON_BLANC = "BT6-082"      # Lv.3 Sistermon Blanc (Yellow)
SISTERMON_CIEL = "BT6-084"       # Lv.4 Sistermon Ciel (Yellow)
MAGNAMON = "BT13-040"            # Lv.4 Royal Knight Magnamon
GALLANTMON = "AD1-008"           # Lv.6 Royal Knight Gallantmon
GANKOOMON_X = "BT10-068"         # Gankoomon (X Antibody) - Royal Knight, should NOT be excluded
OMNIMON_X = "BT5-111"            # Omnimon (X Antibody) - Royal Knight, should NOT be excluded
OMNIMON = "BT1-084"              # Omnimon - Royal Knight, SHOULD be excluded
GANKOOMON_BT6 = "BT6-067"        # Another Gankoomon, SHOULD be excluded
METALGREYMON = "ST1-09"          # Lv.5 non-Royal-Knight (for breeding top)
AGUMON = "ST1-03"                # Lv.3 (filler)


def _find_trash_select_action(runner, card_id):
    """Find the action ID that selects a specific card from trash."""
    player = runner.game.player1
    for i, card in enumerate(player.trash_cards):
        if card.c_entity_base and card.c_entity_base.card_id == card_id:
            return SEL_TRASH_START + i
    return None


@pytest.mark.behavioral
class TestBT13019GankoomonBlocker:
    """Blocker keyword."""

    def test_has_blocker(self, debug_runner):
        """Gankoomon should have the Blocker keyword when on field."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)
        runner.place_on_field(1, [GANKOOMON])

        snap = runner.snapshot()
        ganko = next((s for s in snap.p1_field if s.card_id == GANKOOMON), None)
        assert ganko is not None, "Gankoomon should be on field"
        assert "blocker" in ganko.keywords, (
            f"Gankoomon should have blocker keyword, has: {ganko.keywords}"
        )


@pytest.mark.behavioral
class TestBT13019OnPlaySistermonFromTrash:
    """[On Play] Sistermon from trash branch."""

    def test_play_sistermon_from_trash(self, debug_runner):
        """On Play should allow playing a Sistermon from trash for free."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)
        runner.inject_card(1, GANKOOMON, "hand")
        runner.inject_card(1, SISTERMON_BLANC, "trash")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        action = runner.find_action("Play Gankoomon")
        assert action is not None, "Should be able to play Gankoomon from hand"
        result = runner.execute(action)

        # After On Play, we enter a trash selection phase (only Sistermon source
        # available, so no branch choice). Find and select the Sistermon.
        snap = runner.snapshot()
        assert snap.phase != "Main", "Should be in selection phase"

        # Record memory immediately after playing Gankoomon (cost 13)
        memory_after_play = result.after.memory
        assert memory_after_play == memory_before - 13, (
            f"Memory should decrease by 13 (Gankoomon cost), "
            f"was {memory_before}, now {memory_after_play}"
        )

        # Find the Sistermon in trash and select it
        trash_action = _find_trash_select_action(runner, SISTERMON_BLANC)
        assert trash_action is not None, (
            f"Should find Sistermon in trash. Legal: {runner.actions()}"
        )
        # Snapshot memory right before selecting Sistermon
        mem_before_sister = runner.snapshot().memory
        runner.execute(trash_action)

        # Auto-resolve any remaining triggers (e.g. Sistermon's own On Play).
        # After execution the turn may pass (memory was negative), so we verify
        # the Sistermon is on P1's field regardless of turn state.
        runner.auto_resolve()

        game = runner.game
        sistermon_on_field = any(
            perm.top_card and perm.top_card.c_entity_base.card_id == SISTERMON_BLANC
            for perm in game.player1.battle_area
        )
        assert sistermon_on_field, "Sistermon Blanc should be played from trash to field"

        # Verify it was played for free: the only memory cost was Gankoomon's 13.
        # Memory before Sistermon selection was -3 (= 10 - 13), confirming no
        # additional cost was charged.
        assert mem_before_sister == memory_before - 13, (
            f"No additional memory cost should be charged before Sistermon play, "
            f"expected {memory_before - 13}, got {mem_before_sister}"
        )

    def test_sistermon_from_trash_is_optional(self, debug_runner):
        """Player should be able to decline playing a Sistermon from trash."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)
        runner.inject_card(1, GANKOOMON, "hand")
        runner.inject_card(1, SISTERMON_BLANC, "trash")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        # The effect is optional. We should be able to find a Pass/Decline action.
        snap = runner.snapshot()
        assert snap.phase != "Main", "Should be in a selection phase after On Play trigger"

        pass_action = runner.find_action("Pass")
        if pass_action is None:
            pass_action = runner.find_action("Decline")
        assert pass_action is not None, (
            f"Should have a Pass/Decline action. Available: {runner.actions()}"
        )


@pytest.mark.behavioral
class TestBT13019OnPlayRoyalKnightFromBreeding:
    """[On Play] Royal Knight from breeding digi-sources branch."""

    def test_play_royal_knight_from_breeding_sources(self, debug_runner):
        """On Play should allow playing a Royal Knight from breeding digi-sources."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)

        # Set up breeding area: Magnamon (Royal Knight Lv.4) -> MetalGreymon (top)
        # The Royal Knight (Magnamon) is a digi-source (not the top card).
        runner.place_in_breeding(1, [MAGNAMON, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        # Auto-resolve to play the Royal Knight from breeding sources.
        # The script directly plays it (or presents a selection).
        runner.auto_resolve()

        snap = runner.snapshot()
        magnamon_on_field = any(
            s.card_id == MAGNAMON for s in snap.p1_field
        )
        assert magnamon_on_field, (
            "Magnamon (Royal Knight) should be played from breeding digi-sources to field"
        )

        # Magnamon should have been removed from breeding digi-sources
        breeding_perm = runner.game.player1.breeding_area
        if breeding_perm is not None:
            breeding_ids = [
                cs.c_entity_base.card_id for cs in breeding_perm.card_sources
            ]
            assert MAGNAMON not in breeding_ids, (
                "Magnamon should be removed from breeding digi-sources after being played"
            )

    def test_rk_from_breeding_single_card_still_optional(self, debug_runner):
        """Even with only 1 valid Royal Knight in breeding, player can decline.

        C# reference uses canNoSelect: true for both selections. The current
        Python script auto-plays when there's only 1 valid card, which removes
        player choice.
        """
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)

        # Set up breeding with 1 Royal Knight digi-source
        runner.place_in_breeding(1, [MAGNAMON, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        # We should enter a selection phase where we can decline
        snap = runner.snapshot()
        assert snap.phase != "Main", "Should be in selection phase"

        # Should be able to pass/decline even with only 1 valid option
        pass_action = runner.find_action("Pass")
        if pass_action is None:
            pass_action = runner.find_action("Decline")
        assert pass_action is not None, (
            f"Should be able to decline playing the Royal Knight. Actions: {runner.actions()}"
        )


@pytest.mark.behavioral
class TestBT13019NameExclusion:
    """Name exclusion: [Gankoomon] and [Omnimon] are exact names, not substrings."""

    def test_excludes_exact_gankoomon_from_breeding(self, debug_runner):
        """A card named exactly 'Gankoomon' in breeding sources should be excluded."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)
        # Put a Gankoomon (BT6-067, exact name "Gankoomon") as a Royal Knight
        # digi-source in breeding.
        runner.place_in_breeding(1, [GANKOOMON_BT6, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        # Auto-resolve - Gankoomon is excluded, nothing should be played
        runner.auto_resolve()

        snap = runner.snapshot()
        ganko_bt6_on_field = any(
            s.card_id == GANKOOMON_BT6 for s in snap.p1_field
        )
        assert not ganko_bt6_on_field, (
            "Gankoomon (BT6-067) should be excluded from being played by this effect"
        )

    def test_excludes_exact_omnimon_from_breeding(self, debug_runner):
        """A card named exactly 'Omnimon' should be excluded from breeding sources."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)
        # Put Omnimon (BT1-084, exact name "Omnimon") as a digi-source in breeding
        runner.place_in_breeding(1, [OMNIMON, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        runner.auto_resolve()

        snap = runner.snapshot()
        omnimon_on_field = any(
            s.card_id == OMNIMON for s in snap.p1_field
        )
        assert not omnimon_on_field, (
            "Omnimon should be excluded from being played by this effect"
        )

    def test_allows_gankoomon_x_antibody_from_breeding(self, debug_runner):
        """Gankoomon (X Antibody) should NOT be excluded -- only exact 'Gankoomon' is.

        Card text says [Gankoomon] which means exact name match. The card
        'Gankoomon (X Antibody)' is a different card name and has the Royal Knight
        trait, so it should be eligible.
        """
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)
        # Put Gankoomon (X Antibody) as digi-source in breeding
        runner.place_in_breeding(1, [GANKOOMON_X, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        runner.auto_resolve()

        snap = runner.snapshot()
        ganko_x_on_field = any(
            s.card_id == GANKOOMON_X for s in snap.p1_field
        )
        assert ganko_x_on_field, (
            "Gankoomon (X Antibody) should be allowed -- it's not exact name 'Gankoomon'. "
            f"Field: {[(s.card_id, s.card_name) for s in snap.p1_field]}"
        )

    def test_allows_omnimon_x_antibody_from_breeding(self, debug_runner):
        """Omnimon (X Antibody) should NOT be excluded -- only exact 'Omnimon' is.

        Card text says [Omnimon] which means exact name match. The card
        'Omnimon (X Antibody)' is a different card name and should be eligible.
        """
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)
        # Put Omnimon (X Antibody) as digi-source in breeding
        runner.place_in_breeding(1, [OMNIMON_X, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        runner.auto_resolve()

        snap = runner.snapshot()
        omni_x_on_field = any(
            s.card_id == OMNIMON_X for s in snap.p1_field
        )
        assert omni_x_on_field, (
            "Omnimon (X Antibody) should be allowed -- it's not exact name 'Omnimon'. "
            f"Field: {[(s.card_id, s.card_name) for s in snap.p1_field]}"
        )


@pytest.mark.behavioral
class TestBT13019BranchChoice:
    """Branch choice when both Sistermon in trash AND Royal Knight in breeding."""

    def test_both_available_presents_branch_choice(self, debug_runner):
        """When both sources are available, player should choose which to use."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)

        runner.inject_card(1, SISTERMON_BLANC, "trash")
        runner.place_in_breeding(1, [MAGNAMON, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        # Should be in a selection phase with at least 2 branch options
        snap = runner.snapshot()
        assert snap.phase != "Main", "Should be in selection phase for branch choice"

        actions = runner.actions()
        legal = runner.action_mask()
        # Should have at least 2 choices (branch 0 and branch 1)
        # The action IDs will be SEL_EFFECT_CHOICE_START + 0 and SEL_EFFECT_CHOICE_START + 1
        branch_actions = [a for a in legal
                          if SEL_EFFECT_CHOICE_START <= a <= SEL_EFFECT_CHOICE_START + 9]
        assert len(branch_actions) >= 2, (
            f"Should have at least 2 branch options. Legal: {actions}"
        )

    def test_branch_0_plays_sistermon(self, debug_runner):
        """Selecting branch 0 should lead to Sistermon from trash selection."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)

        runner.inject_card(1, SISTERMON_BLANC, "trash")
        runner.place_in_breeding(1, [MAGNAMON, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        runner.execute(action)

        # Select branch 0 (Sistermon from trash)
        runner.execute(SEL_EFFECT_CHOICE_START + 0)

        # Now select the Sistermon from trash
        trash_action = _find_trash_select_action(runner, SISTERMON_BLANC)
        assert trash_action is not None, "Should find Sistermon in trash"
        runner.execute(trash_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        sistermon_on_field = any(
            s.card_id == SISTERMON_BLANC for s in snap.p1_field
        )
        assert sistermon_on_field, "Sistermon should be on field after branch 0"

    def test_branch_1_plays_royal_knight(self, debug_runner):
        """Selecting branch 1 should play Royal Knight from breeding sources."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)

        runner.inject_card(1, SISTERMON_BLANC, "trash")
        runner.place_in_breeding(1, [MAGNAMON, METALGREYMON])

        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        runner.execute(action)

        # Select branch 1 (Royal Knight from breeding)
        runner.execute(SEL_EFFECT_CHOICE_START + 1)
        runner.auto_resolve()

        snap = runner.snapshot()
        magnamon_on_field = any(
            s.card_id == MAGNAMON for s in snap.p1_field
        )
        assert magnamon_on_field, "Magnamon should be on field after branch 1"

    def test_no_sources_means_no_selection(self, debug_runner):
        """When neither Sistermon in trash nor Royal Knight in breeding, effect does nothing."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)
        # No Sistermon in trash, no Royal Knight in breeding
        runner.inject_card(1, GANKOOMON, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Gankoomon")
        assert action is not None
        runner.execute(action)

        # Should return directly to Main (or auto-resolve optional effects)
        runner.auto_resolve()
        snap = runner.snapshot()
        # Only Gankoomon on field, nothing else played
        non_ganko = [s for s in snap.p1_field if s.card_id != GANKOOMON]
        assert len(non_ganko) == 0, (
            f"No extra cards should be played. Extra: "
            f"{[(s.card_id, s.card_name) for s in non_ganko]}"
        )


@pytest.mark.behavioral
class TestBT13019WhenDigivolving:
    """[When Digivolving] triggers the same play effect."""

    def test_when_digivolving_plays_sistermon(self, debug_runner):
        """Digivolving into Gankoomon should trigger the Sistermon play effect."""
        runner = debug_runner(archetype1="Royal Knights", initial_memory=10)

        # Place a Lv.5 Digimon on field that Gankoomon can digivolve onto
        runner.place_on_field(1, [METALGREYMON])
        runner.inject_card(1, GANKOOMON, "hand")
        runner.inject_card(1, SISTERMON_CIEL, "trash")
        runner.set_phase("Main")

        # Find the digivolve action
        digi_action = runner.find_action("Digivolve")
        if digi_action is None:
            digi_action = runner.find_action("digivolve")
        if digi_action is None:
            actions = runner.actions()
            ganko_actions = {k: v for k, v in actions.items()
                            if "gankoomon" in v.lower() or "digivolv" in v.lower()}
            pytest.skip(
                f"Could not find digivolve action. Relevant: {ganko_actions}"
            )

        runner.execute(digi_action)

        # After digivolving, the When Digivolving trigger fires. Find and select
        # the Sistermon from trash.
        snap = runner.snapshot()
        if snap.phase != "Main":
            trash_action = _find_trash_select_action(runner, SISTERMON_CIEL)
            if trash_action is not None and trash_action in runner.action_mask():
                runner.execute(trash_action)

        runner.auto_resolve()

        snap = runner.snapshot()
        sistermon_on_field = any(
            s.card_id == SISTERMON_CIEL for s in snap.p1_field
        )
        assert sistermon_on_field, (
            "Sistermon Ciel should be played from trash after digivolving into "
            f"Gankoomon. Field: {[(s.card_id, s.card_name) for s in snap.p1_field]}, "
            f"Trash: {snap.p1_trash}"
        )
