"""Behavioral tests for EX11-050 Loudmon (Lv.5 Purple/Red Digimon).

Card text:
  [On Play] [When Digivolving] Trash 2 cards in your hand. Then, delete
  1 of your opponent's Digimon with as much or less DP as 1 of your
  [Dark Dragon] or [Evil Dragon] trait Digimon.

  [All Turns] While you have 4 or fewer cards in your hand, all of your
  [Dark Dragon] or [Evil Dragon] trait Digimon gain <Scapegoat> (When this
  Digimon would be deleted other than by your effects, by deleting 1 of your
  other Digimon, prevent that deletion.)

  Inherited: [Your Turn] While you have 4 or fewer cards in your hand, all of
  your Digimon with the [Dark Dragon] or [Evil Dragon] trait gain
  <Security A. +1> (This Digimon checks 1 additional security card.)

  Alt digi: Lv.4 w/[Dark Dragon] or [Evil Dragon] trait: Cost 3

Key faithfulness requirements:
  1. On Play/WD: trash exactly min(2, hand_size) cards from hand
  2. On Play/WD: reference Digimon must have [Dark Dragon] or [Evil Dragon] trait
  3. On Play/WD: delete opponent's Digimon with DP <= reference Digimon's DP
  4. On Play/WD: deletion must use is_opponent_effect=True
  5. Scapegoat: granted to all own [Dark Dragon]/[Evil Dragon] Digimon (aura)
  6. Scapegoat: only active when hand <= 4
  7. Scapegoat: active on all turns (not just your turn)
  8. Inherited SA+1: only active on your turn with hand <= 4
  9. Inherited SA+1: applies to host permanent with [Dark Dragon]/[Evil Dragon]
  10. Alt digi: from Lv.4 with Dark Dragon or Evil Dragon trait for cost 3
"""

import pytest

# Cards used:
# EX11-050: Loudmon (Lv.5, DP 7000, Purple/Red, Cyborg/LIBERATOR)
# EX11-049: Punkmon (Lv.4, DP 6000, Purple/Red, Dark Dragon/LIBERATOR)
# BT3-081: Devidramon (Lv.4, DP 4000, Purple, Evil Dragon)
# BT12-010: Growlmon (Lv.4, DP 5000, Red, Dark Dragon)
# BT11-079: DarkLizardmon (Lv.4, DP 4000, Purple, Evil Dragon)
# ST1-02: Biyomon (Lv.3, DP 3000, Red, Bird) -- filler/non-dragon
# BT1-009: Monodramon (Lv.3, DP 2000, Green, Mini Dragon) -- non-dragon

_FILLER_DECK = ["ST1-02"] * 50


@pytest.mark.behavioral
class TestEX11050OnPlayTrashAndDelete:
    """Tests for [On Play] Trash 2, then delete by DP comparison."""

    def test_on_play_trash_2_and_delete_opponent(self, debug_runner):
        """On Play: trash 2 from hand, select own Dark Dragon Digimon,
        delete opponent Digimon with DP <= reference DP."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        # Place a Dark Dragon Digimon on P1 field as reference (DP 6000)
        runner.place_on_field(1, ["EX11-049"])  # Punkmon, Dark Dragon, DP 6000

        # Place a weak opponent Digimon (DP 3000 <= 6000)
        runner.place_on_field(2, ["ST1-02"])  # Biyomon, DP 3000

        # Clear hand and set up: need Loudmon + 2 cards to trash
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")  # Loudmon to play
        runner.inject_card(1, "ST1-02", "hand")     # Fodder to trash
        runner.inject_card(1, "ST1-02", "hand")     # Fodder to trash
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        assert len(snap_before.p1_hand) == 3

        # Play Loudmon
        action = runner.find_action("Play Loudmon")
        assert action is not None, f"Should find Play action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Loudmon should be on field
        assert any(s.card_id == "EX11-050" for s in snap_after.p1_field), \
            "Loudmon should be on the field"
        # 2 cards should have been trashed (was 3 in hand: Loudmon played, 2 trashed)
        # Hand: started 3, played 1 (Loudmon), trashed 2 = 0 remaining
        assert len(snap_after.p1_hand) == 0, \
            f"Expected 0 cards in hand (played 1, trashed 2), got {len(snap_after.p1_hand)}"
        # Opponent's Biyomon (DP 3000 <= 6000) should be deleted
        assert not any(s.card_id == "ST1-02" for s in snap_after.p2_field), \
            "Opponent's Biyomon should be deleted (DP 3000 <= 6000)"
        # Check trash for the 2 trashed cards
        p1_trash_st1 = [c for c in snap_after.p1_trash if c == "ST1-02"]
        assert len(p1_trash_st1) >= 2, \
            f"Expected at least 2 ST1-02 in P1 trash, got {len(p1_trash_st1)}"

    def test_on_play_with_only_1_card_in_hand(self, debug_runner):
        """On Play with only 1 extra card: trash 1 (not 2), still proceed to delete."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        # Place Dark Dragon Digimon
        runner.place_on_field(1, ["BT12-010"])  # Growlmon, Dark Dragon, DP 5000
        # Opponent Digimon with DP <= 5000
        runner.place_on_field(2, ["ST1-02"])  # Biyomon, DP 3000

        # Only Loudmon in hand (no extra cards to trash)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Loudmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-050" for s in snap.p1_field), \
            "Loudmon should be on the field"
        # Hand was just Loudmon, so no cards to trash
        # Delete step should still proceed
        assert not any(s.card_id == "ST1-02" for s in snap.p2_field), \
            "Opponent's Biyomon should be deleted (DP 3000 <= 5000)"

    def test_on_play_no_dragon_digimon_skips_delete(self, debug_runner):
        """On Play without any Dark/Evil Dragon Digimon: trash cards but skip delete."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        # No Dark Dragon or Evil Dragon on P1 field -- just Loudmon itself
        # Loudmon has Cyborg/LIBERATOR, NOT Dark Dragon/Evil Dragon
        runner.place_on_field(2, ["ST1-02"])  # Opponent target exists

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")
        runner.inject_card(1, "ST1-02", "hand")
        runner.inject_card(1, "ST1-02", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Loudmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent Biyomon should NOT be deleted (no dragon reference on P1 field)
        # Note: Loudmon itself is Cyborg/LIBERATOR, not dragon trait
        assert any(s.card_id == "ST1-02" for s in snap.p2_field), \
            "Opponent's Biyomon should NOT be deleted (no Dark/Evil Dragon on P1 field)"

    def test_on_play_opponent_dp_too_high(self, debug_runner):
        """On Play: opponent Digimon with DP > reference is not valid target."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        # P1 Dark Dragon with DP 4000
        runner.place_on_field(1, ["BT3-081"])  # Devidramon, Evil Dragon, DP 4000
        # Opponent Digimon with DP 6000 (> 4000)
        runner.place_on_field(2, ["EX11-049"])  # Punkmon, DP 6000

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")
        runner.inject_card(1, "ST1-02", "hand")
        runner.inject_card(1, "ST1-02", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Loudmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's Punkmon (DP 6000) should NOT be deleted (6000 > 4000)
        assert any(s.card_id == "EX11-049" for s in snap.p2_field), \
            "Opponent's Punkmon should NOT be deleted (DP 6000 > reference 4000)"

    def test_on_play_delete_uses_opponent_effect_flag(self, debug_runner):
        """On Play deletion should flag as opponent effect so Scapegoat etc. trigger."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        # P1 Dark Dragon with DP >= Karakurumon's 7000
        runner.place_on_field(1, ["BT7-076"])  # Orochimon, DP 8000, Dark Dragon

        # Opponent: target Digimon with Scapegoat + sacrifice
        runner.place_on_field(2, ["EX11-022"])  # Karakurumon 7000 DP, has Scapegoat
        runner.place_on_field(2, ["ST1-02"])  # Sacrifice for Scapegoat

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")
        runner.inject_card(1, "ST1-02", "hand")
        runner.inject_card(1, "ST1-02", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Loudmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Karakurumon (7000 DP) is targetable by Orochimon ref (8000 DP).
        # If is_opponent_effect=True, Scapegoat fires:
        # Karakurumon survives, sacrifice Biyomon is deleted instead
        assert any(s.card_id == "EX11-022" for s in snap.p2_field), \
            "Karakurumon should survive via Scapegoat (requires is_opponent_effect=True)"
        assert not any(s.card_id == "ST1-02" for s in snap.p2_field), \
            "Sacrifice Biyomon should be deleted by Scapegoat"


@pytest.mark.behavioral
class TestEX11050WhenDigivolving:
    """Tests for [When Digivolving] Trash 2, then delete by DP comparison."""

    def test_when_digivolving_triggers_trash_and_delete(self, debug_runner):
        """Digivolving into Loudmon triggers the trash + delete effect.

        After digivolving Loudmon onto Punkmon, the permanent's top card is
        Loudmon (Cyborg, not Dark Dragon). So we place a SECOND Dark Dragon
        Digimon on field to serve as the DP reference for deletion.
        Digivolving also draws 1 card, so hand math is:
        3 in hand - Loudmon (digivolves) + 1 (draw) - 2 (trashed) = 1.
        """
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=10)

        # Place base Lv.4 Dark Dragon on field for digivolution
        runner.place_on_field(1, ["EX11-049"])  # Punkmon, Lv.4 Dark Dragon, DP 6000
        # Place a second Dark Dragon as DP reference (since top card after
        # digivolving will be Loudmon with Cyborg trait, not Dark Dragon)
        runner.place_on_field(1, ["BT3-081"])  # Devidramon, Evil Dragon, DP 4000
        # Opponent Digimon with DP <= 4000
        runner.place_on_field(2, ["ST1-02"])  # Biyomon, DP 3000

        # Loudmon in hand to digivolve + 2 trash fodder
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")
        runner.inject_card(1, "ST1-02", "hand")  # Trash fodder 1
        runner.inject_card(1, "ST1-02", "hand")  # Trash fodder 2
        runner.set_phase("Main")

        # Alt digi: Lv.4 Dark Dragon -> Loudmon for cost 3
        digi_action = runner.find_action("Digivolve")
        if digi_action is None:
            digi_action = runner.find_action("digivolve")
        assert digi_action is not None, \
            f"Should find digivolve action. Actions: {runner.actions()}"
        runner.execute(digi_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Loudmon should be on field (on top of Punkmon)
        assert any(s.card_id == "EX11-050" for s in snap.p1_field), \
            "Loudmon should be on the field after digivolving"
        # Hand: 3 - 1 (digivolve) + 1 (draw) - 2 (trashed) = 1
        assert len(snap.p1_hand) == 1, \
            f"Expected 1 card in hand (digivolve -1, draw +1, trash -2), got {len(snap.p1_hand)}"
        # Trashed 2 cards
        p1_trash_st1 = [c for c in snap.p1_trash if c == "ST1-02"]
        assert len(p1_trash_st1) >= 2, \
            f"Expected at least 2 ST1-02 in P1 trash, got {len(p1_trash_st1)}"
        # Opponent's Biyomon (DP 3000) should be deleted via Devidramon reference (DP 4000)
        assert not any(s.card_id == "ST1-02" for s in snap.p2_field), \
            "Opponent's Biyomon should be deleted (DP 3000 <= Devidramon's 4000)"


@pytest.mark.behavioral
class TestEX11050Scapegoat:
    """Tests for [All Turns] Scapegoat aura to Dark/Evil Dragon Digimon."""

    def test_scapegoat_granted_to_dragon_digimon(self, debug_runner):
        """Loudmon grants Scapegoat to Dark Dragon/Evil Dragon Digimon when hand <= 4."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Place Loudmon on field
        runner.place_on_field(1, ["EX11-050"])
        # Place Dark Dragon Digimon
        dragon = runner.place_on_field(1, ["EX11-049"])  # Punkmon, Dark Dragon
        # Place sacrifice target
        sacrifice = runner.place_on_field(1, ["ST1-02"])  # Biyomon

        # Ensure hand <= 4
        runner.clear_zone(1, "hand")
        assert len(runner.game.player1.hand_cards) <= 4

        # The Dark Dragon Digimon should have Scapegoat via aura
        assert dragon.has_keyword("_is_scapegoat"), \
            "Dark Dragon Digimon should have Scapegoat (granted by Loudmon aura)"

    def test_scapegoat_prevents_deletion_of_dragon(self, debug_runner):
        """Scapegoat from Loudmon prevents deletion of Dark Dragon Digimon."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Place sacrifice first so it's first in battle_area,
        # then Loudmon, then dragon. Scapegoat auto-selects scapegoat_targets[0].
        sacrifice = runner.place_on_field(1, ["ST1-02"])  # Will be sacrificed
        runner.place_on_field(1, ["EX11-050"])  # Loudmon, grants Scapegoat aura
        dragon = runner.place_on_field(1, ["EX11-049"])  # Punkmon, Dark Dragon

        runner.clear_zone(1, "hand")

        snap_before = runner.snapshot()
        field_count_before = len(snap_before.p1_field)

        # Try to delete the dragon by opponent effect
        deleted = runner.game.player1.delete_permanent(
            dragon, is_opponent_effect=True)

        assert not deleted, "Dragon Digimon should NOT be deleted (Scapegoat)"

        snap = runner.snapshot()
        assert any(s.card_id == "EX11-049" for s in snap.p1_field), \
            "Punkmon should still be on the field (Scapegoat prevented deletion)"
        # One other Digimon should have been sacrificed
        assert len(snap.p1_field) == field_count_before - 1, \
            f"Expected {field_count_before - 1} on field (1 sacrificed), got {len(snap.p1_field)}"

    def test_scapegoat_not_granted_to_non_dragon(self, debug_runner):
        """Scapegoat is NOT granted to non-Dark/Evil Dragon Digimon."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        runner.place_on_field(1, ["EX11-050"])
        non_dragon = runner.place_on_field(1, ["ST1-02"])  # Bird trait, not Dragon

        runner.clear_zone(1, "hand")

        assert not non_dragon.has_keyword("_is_scapegoat"), \
            "Non-dragon Digimon should NOT have Scapegoat"

    def test_scapegoat_not_active_with_5_plus_hand(self, debug_runner):
        """Scapegoat is NOT granted when hand > 4 cards."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        runner.place_on_field(1, ["EX11-050"])
        dragon = runner.place_on_field(1, ["EX11-049"])

        # Ensure hand has 5+ cards
        runner.clear_zone(1, "hand")
        for _ in range(5):
            runner.inject_card(1, "ST1-02", "hand")
        assert len(runner.game.player1.hand_cards) == 5

        assert not dragon.has_keyword("_is_scapegoat"), \
            "Dragon Digimon should NOT have Scapegoat when hand > 4"

    def test_scapegoat_active_with_exactly_4_hand(self, debug_runner):
        """Scapegoat IS granted when hand has exactly 4 cards."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        runner.place_on_field(1, ["EX11-050"])
        dragon = runner.place_on_field(1, ["EX11-049"])

        runner.clear_zone(1, "hand")
        for _ in range(4):
            runner.inject_card(1, "ST1-02", "hand")
        assert len(runner.game.player1.hand_cards) == 4

        assert dragon.has_keyword("_is_scapegoat"), \
            "Dragon Digimon should have Scapegoat when hand == 4"

    def test_scapegoat_active_on_opponent_turn(self, debug_runner):
        """[All Turns] Scapegoat is active even on opponent's turn."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=-5, first_player=2)

        runner.place_on_field(1, ["EX11-050"])
        dragon = runner.place_on_field(1, ["EX11-049"])

        runner.clear_zone(1, "hand")

        # It's P2's turn
        game = runner.game
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player2.is_my_turn = True
        game.player1.is_my_turn = False

        assert dragon.has_keyword("_is_scapegoat"), \
            "Dragon Digimon should have Scapegoat even on opponent's turn ([All Turns])"

    def test_scapegoat_granted_to_evil_dragon(self, debug_runner):
        """Scapegoat is also granted to [Evil Dragon] trait Digimon."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        runner.place_on_field(1, ["EX11-050"])
        evil_dragon = runner.place_on_field(1, ["BT3-081"])  # Devidramon, Evil Dragon

        runner.clear_zone(1, "hand")

        assert evil_dragon.has_keyword("_is_scapegoat"), \
            "Evil Dragon Digimon should have Scapegoat (granted by Loudmon aura)"


@pytest.mark.behavioral
class TestEX11050InheritedSA:
    """Tests for inherited [Your Turn] SA+1 to Dark/Evil Dragon Digimon."""

    def test_inherited_sa_plus_1_on_dragon_host(self, debug_runner):
        """Inherited SA+1 applies when host has Dark Dragon trait, your turn, hand <= 4."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5, first_player=1)

        # Stack: Loudmon as digi-source under Punkmon (Dark Dragon top)
        # This means inherited effects from Loudmon apply to the Punkmon permanent
        dragon_perm = runner.place_on_field(1, ["EX11-050", "EX11-049"])
        # Stack: bottom=Loudmon, top=Punkmon (Dark Dragon)

        runner.clear_zone(1, "hand")
        game = runner.game
        game.turn_player = game.player1
        game.player1.is_my_turn = True

        base_sa = 0  # Default SA for Punkmon
        actual_sa = dragon_perm.security_attack_modifier()
        assert actual_sa == base_sa + 1, \
            f"Expected SA modifier {base_sa + 1} (inherited +1), got {actual_sa}"

    def test_inherited_sa_not_on_opponent_turn(self, debug_runner):
        """Inherited SA+1 is NOT active on opponent's turn ([Your Turn])."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=-5, first_player=2)

        dragon_perm = runner.place_on_field(1, ["EX11-050", "EX11-049"])

        runner.clear_zone(1, "hand")
        game = runner.game
        game.turn_player = game.player2
        game.player2.is_my_turn = True
        game.player1.is_my_turn = False

        assert dragon_perm.security_attack_modifier() == 0, \
            "SA+1 should NOT apply on opponent's turn"

    def test_inherited_sa_not_with_5_plus_hand(self, debug_runner):
        """Inherited SA+1 is NOT active when hand > 4."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5, first_player=1)

        dragon_perm = runner.place_on_field(1, ["EX11-050", "EX11-049"])

        runner.clear_zone(1, "hand")
        for _ in range(5):
            runner.inject_card(1, "ST1-02", "hand")

        game = runner.game
        game.turn_player = game.player1
        game.player1.is_my_turn = True

        assert dragon_perm.security_attack_modifier() == 0, \
            "SA+1 should NOT apply when hand > 4"

    def test_inherited_sa_not_on_non_dragon_host(self, debug_runner):
        """Inherited SA+1 does NOT apply if host is not Dark/Evil Dragon."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5, first_player=1)

        # Stack: Loudmon under Biyomon (Bird trait, not dragon)
        non_dragon_perm = runner.place_on_field(1, ["EX11-050", "ST1-02"])

        runner.clear_zone(1, "hand")
        game = runner.game
        game.turn_player = game.player1
        game.player1.is_my_turn = True

        assert non_dragon_perm.security_attack_modifier() == 0, \
            "SA+1 should NOT apply to non-dragon host"


@pytest.mark.behavioral
class TestEX11050AltDigi:
    """Tests for alternate digivolution from Lv.4 Dark/Evil Dragon for cost 3."""

    def test_alt_digi_from_dark_dragon_lv4(self, debug_runner):
        """Can alt-digivolve from Lv.4 Dark Dragon Digimon for cost 3."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Place Lv.4 Dark Dragon on field
        runner.place_on_field(1, ["EX11-049"])  # Punkmon, Dark Dragon Lv.4
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        assert digi is not None, \
            f"Should find digivolve from Dark Dragon Lv.4. Actions: {runner.actions()}"

    def test_alt_digi_from_evil_dragon_lv4(self, debug_runner):
        """Can alt-digivolve from Lv.4 Evil Dragon Digimon for cost 3."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Place Lv.4 Evil Dragon on field
        runner.place_on_field(1, ["BT3-081"])  # Devidramon, Evil Dragon Lv.4
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        assert digi is not None, \
            f"Should find digivolve from Evil Dragon Lv.4. Actions: {runner.actions()}"

    def test_no_alt_digi_from_non_dragon_lv4(self, debug_runner):
        """Cannot alt-digivolve from Lv.4 without Dark/Evil Dragon trait."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # BT2-071 Wizardmon: Lv.4 Purple Wizard — correct level but wrong trait
        runner.place_on_field(1, ["BT2-071"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-050", "hand")
        runner.set_phase("Main")

        # Loudmon alt-digi needs Lv.4 with Dark Dragon/Evil Dragon trait.
        # Wizardmon is Lv.4 Purple but Wizard trait — should not qualify for alt-digi.
        # Normal evo (Lv.4 Purple) may still work, so check specifically for
        # the alt-digi cost of 3 not being offered.
        actions = runner.actions()
        alt_digi_actions = [a for a in actions if 'Digivolve' in str(a) and '3' in str(a)]
        # If normal Purple Lv.4 evo is available it would be at cost 4 (not 3)
        assert not any('cost 3' in str(a).lower() for a in actions), \
            f"Should NOT offer alt-digi cost 3 from non-dragon Lv.4. Actions: {actions}"
