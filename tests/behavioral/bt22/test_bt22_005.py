"""Behavioral tests for BT22-005 Tsumemon (Lv.2 Digi-Egg Purple, Traits: Unidentified, CS).

Card text:
  Inherited Effect [Your Turn] [Once Per Turn] When any of your Digimon with
  the [Unidentified] or [CS] trait are played, <Draw 1>.

Key verification:
  - Inherited effect: only active when under a Digimon on the battle area
  - Observer pattern: triggers when OTHER Digimon (not just the one it's under)
    with [Unidentified] or [CS] are played
  - Once per turn
  - Only on your turn
  - Draw 1
  - Only triggers on Digimon play (not Tamer play)
"""

import pytest


# BT22-005 Tsumemon: Lv.2 Digi-Egg, Purple, [Unidentified, CS]
TSUMEMON = "BT22-005"
# BT22-008 Agumon: Red Lv.3 Digimon, cost 3, [Reptile, CS]
CS_DIGIMON = "BT22-008"
# BT2-053 Keramon: Purple Lv.3, cost 3, [Unidentified]
UNIDENTIFIED_DIGIMON = "BT2-053"
# ST1-03 Agumon: Red Lv.3, cost 3, [Reptile] — no CS or Unidentified trait
NO_TRAIT_DIGIMON = "ST1-03"
# BT22-094 Yuugo Kamishiro: White Tamer, cost 3, [CS]
CS_TAMER = "BT22-094"


@pytest.mark.behavioral
class TestBT22005Inherited:
    """Tests for BT22-005 Tsumemon inherited effect."""

    def test_draw_on_cs_digimon_play(self, debug_runner):
        """Playing a CS Digimon triggers Tsumemon inherited Draw 1."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        # Place a Digimon with Tsumemon under it on field
        # Tsumemon (Lv.2 egg) under a Lv.3 Digimon
        perm = runner.place_on_field(1, [TSUMEMON, NO_TRAIT_DIGIMON])

        # Put a CS Digimon in hand to play
        runner.inject_card(1, CS_DIGIMON, "hand")
        runner.set_phase("Main")

        hand_before = len(player.hand_cards)
        lib_before = len(player.library_cards)

        play = runner.find_action("Agumon")
        if play is None:
            play = runner.find_action(CS_DIGIMON)
        assert play is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(play)
        runner.auto_resolve()

        # Hand: lost 1 (played CS Digimon) + gained 1 (Draw 1) = net 0
        assert len(player.hand_cards) == hand_before - 1 + 1, (
            f"Should have drawn 1 card from Tsumemon inherited effect. "
            f"Hand before: {hand_before}, after: {len(player.hand_cards)}"
        )
        assert len(player.library_cards) == lib_before - 1, (
            f"Library should have 1 fewer card (drawn). "
            f"Before: {lib_before}, after: {len(player.library_cards)}"
        )

    def test_draw_on_unidentified_digimon_play(self, debug_runner):
        """Playing an Unidentified Digimon triggers Tsumemon inherited Draw 1."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        # Place a Digimon with Tsumemon under it
        perm = runner.place_on_field(1, [TSUMEMON, NO_TRAIT_DIGIMON])

        runner.inject_card(1, UNIDENTIFIED_DIGIMON, "hand")
        runner.set_phase("Main")

        hand_before = len(player.hand_cards)
        lib_before = len(player.library_cards)

        play = runner.find_action("Keramon")
        if play is None:
            play = runner.find_action(UNIDENTIFIED_DIGIMON)
        assert play is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(play)
        runner.auto_resolve()

        # Hand: lost 1 (played) + gained 1 (draw) = net 0
        assert len(player.hand_cards) == hand_before - 1 + 1, (
            f"Should have drawn 1 from Tsumemon inherited. "
            f"Hand before: {hand_before}, after: {len(player.hand_cards)}"
        )

    def test_no_draw_on_non_matching_digimon_play(self, debug_runner):
        """Playing a Digimon without [Unidentified] or [CS] does NOT trigger draw."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, [TSUMEMON, CS_DIGIMON])

        runner.inject_card(1, NO_TRAIT_DIGIMON, "hand")
        runner.set_phase("Main")

        hand_before = len(player.hand_cards)
        lib_before = len(player.library_cards)

        play = runner.find_action("Play")
        if play is None:
            play = runner.find_action(NO_TRAIT_DIGIMON)
        assert play is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(play)
        runner.auto_resolve()

        # Hand: lost 1 (played), no draw = net -1
        assert len(player.hand_cards) == hand_before - 1, (
            f"Should NOT have drawn (no matching trait). "
            f"Hand before: {hand_before}, after: {len(player.hand_cards)}"
        )
        assert len(player.library_cards) == lib_before, (
            f"Library should be unchanged. Before: {lib_before}, after: {len(player.library_cards)}"
        )

    def test_once_per_turn(self, debug_runner):
        """Tsumemon inherited effect only triggers once per turn."""
        runner = debug_runner(initial_memory=15)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, [TSUMEMON, NO_TRAIT_DIGIMON])

        # Put 2 CS Digimon in hand
        runner.inject_card(1, CS_DIGIMON, "hand")
        runner.inject_card(1, CS_DIGIMON, "hand")
        runner.set_phase("Main")

        lib_before = len(player.library_cards)

        # Play first CS Digimon — should trigger draw
        play1 = runner.find_action("Agumon")
        if play1 is None:
            play1 = runner.find_action(CS_DIGIMON)
        assert play1 is not None
        runner.execute(play1)
        runner.auto_resolve()

        lib_after_first = len(player.library_cards)
        assert lib_after_first == lib_before - 1, (
            "First play should have triggered Draw 1"
        )

        # Play second CS Digimon — should NOT trigger draw again (OPT)
        play2 = runner.find_action("Agumon")
        if play2 is None:
            play2 = runner.find_action(CS_DIGIMON)
        assert play2 is not None
        runner.execute(play2)
        runner.auto_resolve()

        lib_after_second = len(player.library_cards)
        assert lib_after_second == lib_after_first, (
            f"Second play should NOT trigger draw (once per turn). "
            f"After first: {lib_after_first}, after second: {lib_after_second}"
        )

    def test_not_inherited_on_own_play(self, debug_runner):
        """If Tsumemon is just in egg (not under a Digimon on field), no trigger."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        # Tsumemon in breeding area (not on battle area, so inherited not active)
        runner.place_in_breeding(1, [TSUMEMON])

        runner.inject_card(1, CS_DIGIMON, "hand")
        runner.set_phase("Main")

        lib_before = len(player.library_cards)

        play = runner.find_action("Agumon")
        if play is None:
            play = runner.find_action(CS_DIGIMON)
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        # No draw since inherited is not active from breeding area
        assert len(player.library_cards) == lib_before, (
            "Inherited effect should NOT fire from breeding area"
        )
