"""Behavioral tests for BT12-031 Imperialdramon: Fighter Mode.

Card text:
  Lv.6 | Blue/Green | Ancient Dragonkin | DP:13000 | Cost:13
  Evo: Blue Lv.5 for cost 5
  Alt-Digi: From [Imperialdramon: Dragon Mode] for cost 2

  [When Digivolving] Suspend all of your opponent's Digimon with no
  digivolution cards. Then, return 1 of your opponent's suspended Digimon to
  its owner's hand. By returning 1 [Imperialdramon: Dragon Mode] from this
  Digimon's digivolution cards to its owner's hand, return all of your
  opponent's suspended Digimon at the bottom of their owners' decks instead.

  [All Turns] This Digimon gets +1000 DP for each color in its digivolution
  cards. While there are 2 or more colors in its digivolution cards, it gains
  <Security Attack +1> and <Blocker>.

Key faithfulness points tested:
  - WD: Suspend all opp Digimon with no digi-cards.
  - WD: Then bounce 1 opp suspended Digimon to hand (default path).
  - WD: Optional Dragon Mode path - return Dragon Mode from digi-stack to hand,
        deck-bottom ALL opp suspended Digimon instead of bouncing 1.
  - Static DP: +1000 per unique color across all digi-stack cards.
  - Static SA+1: only when 2+ colors in digi-stack.
  - Static Blocker: only when 2+ colors in digi-stack.
  - Alt-digi: from Imperialdramon: Dragon Mode for cost 2.
"""

import pytest

# Card IDs used in tests
BT12_031 = "BT12-031"   # Imperialdramon: Fighter Mode (Lv.6 Blue/Green)
BT12_030 = "BT12-030"   # Imperialdramon: Dragon Mode (Lv.6 Blue/Green) - for alt-digi and digi-stack
ST9_06 = "ST9-06"       # Imperialdramon Dragon Mode (Lv.6 Blue/Green) - alternative name
ST9_05 = "ST9-05"       # Paildramon (Lv.5 Blue/Green)
ST9_04 = "ST9-04"       # ExVeemon (Lv.4 Blue)
ST9_09 = "ST9-09"       # Stingmon (Lv.4 Green)
ST9_02 = "ST9-02"       # Veemon (Lv.3 Blue)
FILLER = "ST1-03"       # Agumon (generic filler)
RED_LV4 = "ST1-05"      # Birdramon (Red Lv.4 - single color for DP test)


@pytest.mark.behavioral
class TestBT12_031WhenDigivolving:
    """[When Digivolving] Suspend no-digi-cards, bounce/deck-bottom."""

    def test_suspends_opp_digimon_with_no_digi_cards(self, debug_runner):
        """Should suspend all opponent's Digimon that have no digivolution cards."""
        deck = [BT12_031, ST9_05] + [FILLER] * 48
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Place Fighter Mode evo base on P1's field
        perm = runner.place_on_field(1, [ST9_04, ST9_05])
        runner.inject_card(1, BT12_031, "hand")

        # Place opponent's Digimon: one with no digi-cards (single card)
        # and one with digi-cards (stacked)
        opp_single = runner.place_on_field(2, [FILLER])  # no digi-cards
        opp_stacked = runner.place_on_field(2, [ST9_02, ST9_04])  # has digi-cards

        runner.set_phase("Main")

        # Confirm opponent's single-card Digimon is not suspended yet
        snap_before = runner.snapshot()
        opp_single_slot = next(
            (s for s in snap_before.p2_field if s.card_id == FILLER), None
        )
        assert opp_single_slot is not None
        assert not opp_single_slot.is_suspended

        # Digivolve Fighter Mode
        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None, f"Should find digivolve action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # The single-card opponent Digimon should be suspended (or removed by deck-bottom/bounce)
        # Since auto_resolve picks the first action, it might bounce or deck-bottom
        # We need to check: if the single Digimon is still on field, it should be suspended
        remaining_singles = [
            s for s in snap_after.p2_field if s.card_id == FILLER
        ]
        for s in remaining_singles:
            assert s.is_suspended, (
                f"Opponent's no-digi-card Digimon should be suspended. "
                f"Slot: {s.card_id}, suspended={s.is_suspended}"
            )

        # The stacked Digimon should NOT be suspended by this effect
        stacked_slot = next(
            (s for s in snap_after.p2_field if s.card_id == ST9_04), None
        )
        if stacked_slot:
            assert not stacked_slot.is_suspended, (
                f"Stacked opponent Digimon (has digi-cards) should NOT be suspended. "
                f"Slot: {stacked_slot.card_id}, suspended={stacked_slot.is_suspended}"
            )

    def test_bounce_one_suspended_default_path(self, debug_runner):
        """Without Dragon Mode in digi-stack, should bounce 1 suspended opp Digimon to hand."""
        deck = [BT12_031, ST9_05] + [FILLER] * 48
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # No Dragon Mode in digi-stack (just ExVeemon + Paildramon)
        perm = runner.place_on_field(1, [ST9_04, ST9_05])
        runner.inject_card(1, BT12_031, "hand")

        # Two opponent Digimon with no digi-cards (will be suspended)
        runner.place_on_field(2, [FILLER])
        runner.place_on_field(2, [FILLER])

        runner.set_phase("Main")

        snap_before = runner.snapshot()
        opp_field_before = len(snap_before.p2_field)
        opp_hand_before = len(snap_before.p2_hand)

        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # One opponent Digimon should have been bounced to hand
        opp_field_after = len(snap_after.p2_field)
        opp_hand_after = len(snap_after.p2_hand)

        assert opp_field_after == opp_field_before - 1, (
            f"One opponent Digimon should be bounced. "
            f"Field before: {opp_field_before}, after: {opp_field_after}"
        )
        assert opp_hand_after == opp_hand_before + 1, (
            f"One card should be added to opponent's hand (bounced). "
            f"Hand before: {opp_hand_before}, after: {opp_hand_after}"
        )

    def test_dragon_mode_path_deck_bottoms_all_suspended(self, debug_runner):
        """Returning Dragon Mode from digi-stack should deck-bottom ALL suspended opp Digimon."""
        deck = [BT12_031, BT12_030, ST9_05] + [FILLER] * 47
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Stack with Imperialdramon: Dragon Mode underneath
        # Bottom-to-top: Dragon Mode, Paildramon (or just Dragon Mode, Fighter Mode will go on top)
        perm = runner.place_on_field(1, [BT12_030, ST9_05])
        runner.inject_card(1, BT12_031, "hand")

        # Three opponent Digimon with no digi-cards
        runner.place_on_field(2, [FILLER])
        runner.place_on_field(2, [FILLER])
        runner.place_on_field(2, [FILLER])

        runner.set_phase("Main")

        snap_before = runner.snapshot()
        opp_field_before = len(snap_before.p2_field)
        opp_lib_before = snap_before.p2_library_size

        # Digivolve
        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None
        runner.execute(action)

        # The effect should offer a branch choice (return Dragon Mode or don't)
        # We need to pick the Dragon Mode branch (choice 0)
        snap = runner.snapshot()
        if snap.phase != "Main":
            # Look for the branch choice action for "Return Dragon Mode"
            branch_action = runner.find_action("Return Dragon Mode")
            if branch_action is not None:
                runner.execute(branch_action)
                runner.auto_resolve()
            else:
                # auto_resolve will pick first available
                runner.auto_resolve()

        snap_after = runner.snapshot()

        # ALL three opponent Digimon should be removed from field (deck-bottomed)
        assert len(snap_after.p2_field) == 0, (
            f"All suspended opponent Digimon should be placed at deck bottom. "
            f"Remaining on field: {[s.card_id for s in snap_after.p2_field]}"
        )

        # Dragon Mode should be in P1's hand (returned)
        assert BT12_030 in snap_after.p1_hand, (
            f"Dragon Mode should have been returned to P1's hand. "
            f"P1 hand: {snap_after.p1_hand}"
        )

        # Opponent's deck should have grown by 3 (3 cards deck-bottomed)
        opp_lib_after = snap_after.p2_library_size
        assert opp_lib_after == opp_lib_before + 3, (
            f"Opponent's deck should grow by 3 (deck-bottomed Digimon). "
            f"Before: {opp_lib_before}, After: {opp_lib_after}"
        )

    def test_does_not_suspend_digimon_with_digi_cards(self, debug_runner):
        """Opponent's Digimon WITH digivolution cards should NOT be suspended."""
        deck = [BT12_031, ST9_05] + [FILLER] * 48
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        perm = runner.place_on_field(1, [ST9_04, ST9_05])
        runner.inject_card(1, BT12_031, "hand")

        # Stacked opponent Digimon (has digi-cards = has digivolution cards)
        opp_stacked = runner.place_on_field(2, [ST9_02, ST9_04])

        runner.set_phase("Main")

        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # The stacked Digimon should still be on field and not suspended
        stacked = next(
            (s for s in snap_after.p2_field if s.card_id == ST9_04), None
        )
        assert stacked is not None, (
            f"Stacked opponent Digimon should still be on field. "
            f"P2 field: {[s.card_id for s in snap_after.p2_field]}"
        )
        assert not stacked.is_suspended, (
            "Opponent's Digimon with digi-cards should NOT be suspended"
        )


@pytest.mark.behavioral
class TestBT12_031StaticDP:
    """[All Turns] +1000 DP per unique color in digi-stack.

    The DP modifier is a declarative effect using a dynamic dp_modifier
    property that recalculates from the current digi-stack colors.
    """

    def test_dp_boost_with_two_colors(self, debug_runner):
        """With 2 colors in digi-stack, DP should be base + 2000."""
        deck = [BT12_031] + [FILLER] * 49
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Stack: ExVeemon (Blue), Stingmon (Green), Fighter Mode (Blue/Green)
        # Colors: Blue, Green = 2 unique colors
        perm = runner.place_on_field(1, [ST9_04, ST9_09, BT12_031])

        snap = runner.snapshot()
        fighter = next(
            (s for s in snap.p1_field if s.card_id == BT12_031), None
        )
        assert fighter is not None
        # Base DP 13000 + 2 colors * 1000 = 15000
        assert fighter.dp == 15000, (
            f"Fighter Mode with 2 colors in digi-stack should have 15000 DP "
            f"(13000 + 2*1000). Got {fighter.dp}"
        )

    def test_dp_boost_with_three_colors(self, debug_runner):
        """With 3 unique colors in digi-stack, DP should be base + 3000."""
        deck = [BT12_031] + [FILLER] * 49
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Stack: ExVeemon (Blue), Stingmon (Green), Birdramon (Red), Fighter Mode (Blue/Green)
        # Colors: Blue, Green, Red = 3 unique colors
        perm = runner.place_on_field(1, [ST9_04, ST9_09, RED_LV4, BT12_031])

        snap = runner.snapshot()
        fighter = next(
            (s for s in snap.p1_field if s.card_id == BT12_031), None
        )
        assert fighter is not None
        # Base DP 13000 + 3 colors * 1000 = 16000
        assert fighter.dp == 16000, (
            f"Fighter Mode with 3 colors in digi-stack should have 16000 DP "
            f"(13000 + 3*1000). Got {fighter.dp}"
        )


@pytest.mark.behavioral
class TestBT12_031StaticKeywords:
    """SA+1 and Blocker when 2+ colors in digi-stack."""

    def test_blocker_with_two_plus_colors(self, debug_runner):
        """With 2+ colors in digi-stack, should have Blocker."""
        deck = [BT12_031] + [FILLER] * 49
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Stack with 2+ colors
        perm = runner.place_on_field(1, [ST9_04, ST9_09, BT12_031])

        snap = runner.snapshot()
        fighter = next(
            (s for s in snap.p1_field if s.card_id == BT12_031), None
        )
        assert fighter is not None
        assert "blocker" in fighter.keywords, (
            f"Fighter Mode with 2+ digi-stack colors should have Blocker. "
            f"Keywords: {fighter.keywords}"
        )

    def test_security_attack_plus_with_two_plus_colors(self, debug_runner):
        """With 2+ colors in digi-stack, should have Security Attack +1.
        The SA+1 is implemented as a declarative _security_attack_modifier,
        which is read by Permanent.security_attack_modifier() and conditioned
        on 2+ colors. We check via the Permanent directly."""
        deck = [BT12_031] + [FILLER] * 49
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Place with 2+ colors in stack
        perm = runner.place_on_field(1, [ST9_04, ST9_09, BT12_031])

        sa_mod = perm.security_attack_modifier()
        assert sa_mod >= 1, (
            f"Fighter Mode with 2+ digi-stack colors should have SA+1. "
            f"security_attack_modifier() returned {sa_mod}"
        )


@pytest.mark.behavioral
class TestBT12_031AltDigi:
    """Alt-digi: From [Imperialdramon: Dragon Mode] for cost 2."""

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi effect should specify Dragon Mode as source, cost 2."""
        deck = [BT12_031] + [FILLER] * 49
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        perm = runner.place_on_field(1, [BT12_031])
        card = perm.top_card
        effects = card.effect_list(None)

        alt_digi = None
        for eff in effects:
            if hasattr(eff, '_alt_digi_cost') and eff._alt_digi_cost is not None:
                alt_digi = eff
                break

        assert alt_digi is not None, "Should have an alt-digi effect"
        assert alt_digi._alt_digi_cost == 2, (
            f"Alt-digi cost should be 2, got {alt_digi._alt_digi_cost}"
        )
        assert "dragon mode" in alt_digi._alt_digi_name.lower(), (
            f"Alt-digi name should reference Dragon Mode. "
            f"Got: {alt_digi._alt_digi_name}"
        )

    def test_alt_digi_from_dragon_mode_on_field(self, debug_runner):
        """Should be able to digivolve onto Imperialdramon: Dragon Mode for cost 2."""
        deck = [BT12_031, BT12_030] + [FILLER] * 48
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Place Dragon Mode on field
        runner.place_on_field(1, [BT12_030])
        runner.inject_card(1, BT12_031, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        # Should find a digivolve action for Fighter Mode
        action = runner.find_action("Digivolve Imperialdramon: Fighter")
        if action is None:
            action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None, (
            f"Should be able to digivolve Fighter Mode onto Dragon Mode. "
            f"Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        memory_spent = memory_before - snap_after.memory
        assert memory_spent == 2, (
            f"Alt-digi from Dragon Mode should cost 2 memory. "
            f"Spent: {memory_spent} (before={memory_before}, after={snap_after.memory})"
        )
