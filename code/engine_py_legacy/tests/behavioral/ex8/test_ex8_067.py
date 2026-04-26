"""Behavioral tests for EX8-067 Close (Tamer, Black, Play Cost 3).

Card text:
  [Start of Your Turn] If you have 2 or less memory, set it to 3.
  [Your Turn] When any of your Digimon digivolve into a [Mineral] or [Rock]
    trait Digimon, by suspending this Tamer, place up to 2 cards with the
    [Mineral] or [Rock] trait from your trash as that Digimon's bottom
    digivolution cards.
  Security Effect [Security] Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import GamePhase
from engine_py_legacy.engine.game.constants import SEL_TRASH_START

# Filler deck uses a non-Mineral/non-Rock card to avoid interference.
# ST1-04 Dracomon (Lv.3, Dragon trait) is a safe choice.
FILLER_DECK = ["ST1-04"] * 50


def _setup_digivolve_scenario(runner, trash_cards, tamer_suspended=False):
    """Helper to set up the standard digivolve scenario.

    Places EX8-067 tamer + EX8-046 Gotsumon (Rock, Lv.3) on field,
    EX8-048 Landramon (Mineral, Lv.4) in hand, and specified cards in trash.
    Returns the digivolve action ID.
    """
    runner.place_on_field(1, ["EX8-067"], is_suspended=tamer_suspended)
    runner.place_on_field(1, ["EX8-046"])  # Gotsumon Lv.3 Rock (base)
    runner.inject_card(1, "EX8-048", zone="hand")  # Landramon Lv.4 Mineral
    for card_id in trash_cards:
        runner.inject_card(1, card_id, zone="trash")
    digi_action = runner.find_action("Digivolve")
    assert digi_action is not None, (
        f"Should find a Digivolve action. Available: {runner.actions()}"
    )
    return digi_action


def _resolve_to_select_trash(runner, digi_action, max_steps=10):
    """Execute digivolve and resolve until SelectTrash phase.

    Returns True if SelectTrash was reached, False otherwise.
    """
    runner.execute(digi_action)
    game = runner.game
    for _ in range(max_steps):
        if game.current_phase == GamePhase.SelectTrash:
            return True
        if game.current_phase in (GamePhase.Main, GamePhase.End):
            return False
        if game.game_over:
            return False
        legal = runner.action_mask()
        if not legal:
            return False
        # Accept optional effects (first legal action)
        runner.execute(legal[0])
    return game.current_phase == GamePhase.SelectTrash


@pytest.mark.behavioral
class TestEX8067MemorySet:
    """Tests for [Start of Your Turn] memory set to 3."""

    def test_memory_set_to_3_when_2_or_less(self, debug_runner):
        """If memory is 2 or less at start of turn, it should be set to 3."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=1, skip_shuffle=True,
        )
        # Place tamer on field
        runner.place_on_field(1, ["EX8-067"])
        game = runner.game

        # Force memory to 1 and start a new turn
        game.memory = 1
        # Pass turn to P2, then back to P1 to trigger start of turn
        runner.set_phase("Main")
        pass_action = runner.find_action("Pass turn")
        if pass_action is not None:
            runner.execute(pass_action)
            # Auto-resolve P2's turn
            runner.auto_resolve()
            # P2 passes turn
            pass_action2 = runner.find_action("Pass")
            if pass_action2 is not None:
                runner.execute(pass_action2)
                runner.auto_resolve()

        snap = runner.snapshot()
        # After start of P1's turn, memory should be set to 3
        assert snap.memory >= 3, (
            f"Expected memory >= 3 after tamer effect, got {snap.memory}"
        )


@pytest.mark.behavioral
class TestEX8067DigivolveTrashSelection:
    """Tests for [Your Turn] digivolve trigger with trash selection.

    When any of your Digimon digivolve into a [Mineral] or [Rock] trait
    Digimon, by suspending this Tamer, place up to 2 cards with the
    [Mineral] or [Rock] trait from your trash as that Digimon's bottom
    digivolution cards.

    CRITICAL: The selection must be player-driven via SelectTrash phase,
    NOT auto-selected. The C# uses SelectCardEffect with canNoSelect=true,
    maxCount=2, canEndNotMax=true.
    """

    def test_enters_select_trash_phase(self, debug_runner):
        """After digivolving into Mineral/Rock Digimon with tamer on field,
        the game MUST enter SelectTrash phase for player choice.
        This is the core no-approximations test."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["BT4-065", "BT4-066"]
        )

        reached_select_trash = _resolve_to_select_trash(runner, digi_action)
        assert reached_select_trash, (
            f"Must enter SelectTrash phase for player-driven selection. "
            f"Current phase: {runner.snapshot().phase}. "
            f"This fails if cards are auto-selected without player choice."
        )

    def test_player_selects_specific_trash_card(self, debug_runner):
        """Player should be able to select specific Mineral/Rock cards from trash.
        With 3 valid cards in trash, all 3 should be selectable."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["BT4-065", "BT4-066", "BT4-070"]
        )

        reached = _resolve_to_select_trash(runner, digi_action)
        assert reached, (
            f"Must reach SelectTrash. Phase: {runner.snapshot().phase}"
        )

        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert len(trash_actions) == 3, (
            f"Expected 3 selectable Mineral/Rock trash cards, "
            f"got {len(trash_actions)}. Legal: {legal}"
        )

        # Select first card
        runner.execute(trash_actions[0])

        # Should enter another SelectTrash for 2nd selection
        snap2 = runner.snapshot()
        assert snap2.phase == "SelectTrash", (
            f"Should enter 2nd SelectTrash for 2nd card. Phase: {snap2.phase}"
        )

        legal2 = runner.action_mask()
        trash_actions2 = [a for a in legal2 if a >= SEL_TRASH_START]
        assert len(trash_actions2) == 2, (
            f"Expected 2 remaining selectable cards for 2nd pick, "
            f"got {len(trash_actions2)}. Legal: {legal2}"
        )

        # Select 2nd card
        runner.execute(trash_actions2[0])
        runner.auto_resolve()

        # Verify: 2 cards moved from trash to digivolution stack
        snap_final = runner.snapshot()
        digi_perm = [f for f in snap_final.p1_field if f.card_id == "EX8-048"]
        assert digi_perm, "Landramon should be on field"
        # Stack: EX8-046 (base) + EX8-048 (top) + 2 placed cards = 4
        assert len(digi_perm[0].stack_ids) >= 4, (
            f"Expected >= 4 cards in stack (base + top + 2 placed), "
            f"got {len(digi_perm[0].stack_ids)}: {digi_perm[0].stack_ids}"
        )

    def test_decline_on_first_selection(self, debug_runner):
        """Player can decline to place any cards (effect says 'up to 2').
        Action 62 should be available as decline."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["BT4-065"]
        )

        reached = _resolve_to_select_trash(runner, digi_action)
        assert reached, f"Must reach SelectTrash. Phase: {runner.snapshot().phase}"

        legal = runner.action_mask()
        assert 62 in legal, (
            f"Decline (62) should be available for 'up to 2' selection. Legal: {legal}"
        )

        # Decline
        runner.execute(62)
        runner.auto_resolve()

        # Trash card should still be in trash
        snap = runner.snapshot()
        assert "BT4-065" in snap.p1_trash, (
            f"Declined card should remain in trash. Trash: {snap.p1_trash}"
        )

    def test_decline_after_first_selection(self, debug_runner):
        """Player can select 1 card and then decline the 2nd (place only 1)."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["BT4-065", "BT4-066"]
        )

        reached = _resolve_to_select_trash(runner, digi_action)
        assert reached, f"Must reach SelectTrash. Phase: {runner.snapshot().phase}"

        # Select first card
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        runner.execute(trash_actions[0])

        # Should enter 2nd SelectTrash
        snap2 = runner.snapshot()
        assert snap2.phase == "SelectTrash", (
            f"Should enter 2nd SelectTrash. Phase: {snap2.phase}"
        )

        # Decline 2nd selection
        legal2 = runner.action_mask()
        assert 62 in legal2, "Decline should be available for 2nd selection"
        runner.execute(62)
        runner.auto_resolve()

        # Only 1 card should have been placed
        snap_final = runner.snapshot()
        digi_perm = [f for f in snap_final.p1_field if f.card_id == "EX8-048"]
        assert digi_perm, "Landramon should be on field"
        # Stack: EX8-046 (base) + EX8-048 (top) + 1 placed = 3
        assert len(digi_perm[0].stack_ids) == 3, (
            f"Expected 3 cards in stack (base + top + 1 placed), "
            f"got {len(digi_perm[0].stack_ids)}: {digi_perm[0].stack_ids}"
        )

    def test_only_mineral_rock_cards_selectable(self, debug_runner):
        """Only cards with [Mineral] or [Rock] trait should be selectable from trash."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["ST1-04", "BT4-065", "ST1-07"]
        )

        reached = _resolve_to_select_trash(runner, digi_action)
        assert reached, f"Must reach SelectTrash. Phase: {runner.snapshot().phase}"

        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        # Only BT4-065 (Gotsumon Rock) should be selectable
        assert len(trash_actions) == 1, (
            f"Expected exactly 1 selectable Mineral/Rock card, "
            f"got {len(trash_actions)}. Legal: {legal}, Actions: {runner.actions()}"
        )

    def test_tamer_must_be_unsuspended(self, debug_runner):
        """Tamer must be unsuspended to activate the effect (suspend is the cost)."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["BT4-065"], tamer_suspended=True
        )

        runner.execute(digi_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Trash card should remain (effect didn't trigger due to suspended tamer)
        assert "BT4-065" in snap.p1_trash, (
            f"Trash card should remain when tamer is suspended. Trash: {snap.p1_trash}"
        )

    def test_no_trigger_for_non_mineral_rock_digimon(self, debug_runner):
        """Effect should NOT trigger when digivolving into a non-Mineral/Rock Digimon."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        game = runner.game

        runner.place_on_field(1, ["EX8-067"])
        runner.place_on_field(1, ["ST1-04"])  # Dracomon Lv.3 Dragon
        runner.inject_card(1, "ST1-07", zone="hand")  # Greymon Lv.4 Dinosaur
        runner.inject_card(1, "BT4-065", zone="trash")

        digi_action = runner.find_action("Digivolve")
        if digi_action is not None:
            runner.execute(digi_action)
            runner.auto_resolve()

            snap = runner.snapshot()
            tamer_field = [f for f in snap.p1_field if f.card_id == "EX8-067"]
            if tamer_field:
                assert not tamer_field[0].is_suspended, (
                    "Tamer should remain unsuspended when digivolving into "
                    "non-Mineral/Rock Digimon"
                )
            assert "BT4-065" in snap.p1_trash, (
                "Trash should be untouched when digivolving into non-Mineral/Rock"
            )

    def test_cards_placed_at_bottom_of_stack(self, debug_runner):
        """Cards from trash should be placed as BOTTOM digivolution cards."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["BT4-065"]
        )

        reached = _resolve_to_select_trash(runner, digi_action)
        assert reached, f"Must reach SelectTrash. Phase: {runner.snapshot().phase}"

        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert trash_actions, "Should have trash cards to select"
        runner.execute(trash_actions[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        digi_perm = [f for f in snap.p1_field if f.card_id == "EX8-048"]
        assert digi_perm, "Landramon should be on field"
        stack = digi_perm[0].stack_ids
        assert "BT4-065" in stack, (
            f"BT4-065 should be in the digivolution stack. Stack: {stack}"
        )
        # Placed card should be at bottom of stack (index 0)
        assert stack[0] == "BT4-065", (
            f"Placed card should be at bottom of stack. Stack: {stack}"
        )

    def test_no_mineral_rock_in_trash_skips_selection(self, debug_runner):
        """When trash has no Mineral/Rock cards, effect condition should fail."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["ST1-04"]  # Dragon, not Mineral/Rock
        )

        runner.execute(digi_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        tamer_field = [f for f in snap.p1_field if f.card_id == "EX8-067"]
        if tamer_field:
            assert not tamer_field[0].is_suspended, (
                "Tamer should not suspend when no valid Mineral/Rock cards in trash"
            )

    def test_tamer_suspends_as_cost(self, debug_runner):
        """After the effect activates, tamer should be suspended."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        digi_action = _setup_digivolve_scenario(
            runner, trash_cards=["BT4-065"]
        )

        reached = _resolve_to_select_trash(runner, digi_action)
        assert reached, f"Must reach SelectTrash. Phase: {runner.snapshot().phase}"

        # Even before selecting, tamer should be suspended (cost paid)
        snap = runner.snapshot()
        tamer_field = [f for f in snap.p1_field if f.card_id == "EX8-067"]
        assert tamer_field, "Tamer should still be on field"
        assert tamer_field[0].is_suspended, (
            "Tamer should be suspended (cost paid) when entering SelectTrash"
        )
