"""Behavioral tests for BT16-082 Ukkomon (Lv.3 White Digimon, DP 2000, Cost 3).

Card text:
  [Your Turn][Once Per Turn] When one of your Digimon moves from the breeding
  area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon
  card or Tamer card among them to the hand. Return the rest to the bottom of
  the deck. Then, you may hatch in your breeding area.

C# reference:
  - OnMove timing with Once Per Turn
  - PermanentCondition: IsPermanentExistsOnOwnerBattleAreaDigimon
  - RevealDeckTopCardsAndSelect(3, select Digimon/Tamer, rest to deck bottom)
  - Optional hatch if CanHatch
  - Context key for moved permanent: 'moved_permanent' (not 'event_permanent')
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming

# Card IDs
UKKOMON = "BT16-082"
AGUMON = "ST1-03"       # Lv.3 Red Digimon
TAI = "ST1-12"          # Tamer
SHADOW_WING = "ST1-13"  # Option card
YOKOMON = "BT1-001"     # Digi-Egg (Lv.2)
GREYMON = "ST1-07"      # Lv.4 Digimon

# Filler deck needs at least 50 cards (minus digi-eggs)
_FILLER_DECK = [UKKOMON] * 4 + [AGUMON] * 46


@pytest.mark.behavioral
class TestBT16082Ukkomon:
    """Tests for BT16-082 Ukkomon."""

    # ── Effect metadata ───────────────────────────────────────────────

    def test_has_on_move_effect(self, debug_runner):
        """Should have exactly 1 OnMove timing effect."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [UKKOMON])

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove]
        assert len(on_move) == 1, "Should have exactly 1 OnMove effect"

    def test_on_move_is_optional(self, debug_runner):
        """Effect should be optional."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [UKKOMON])

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]
        assert on_move.is_optional, "OnMove effect should be optional"

    def test_on_move_once_per_turn(self, debug_runner):
        """Effect should be once per turn."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [UKKOMON])

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]
        assert on_move.max_count_per_turn == 1, "Should be once per turn"

    # ── Condition checks (unit-level) ─────────────────────────────────

    def test_condition_requires_on_field(self, debug_runner):
        """Condition should require the card to be on the battle area."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)

        # Inject to hand (not on field) -- permanent_of_this_card() returns None
        runner.inject_card(1, UKKOMON, "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]

        # Even with a valid moved_permanent, should fail because Ukkomon isn't on field
        moved_perm = runner.place_on_field(1, [AGUMON])
        ctx = {'moved_permanent': moved_perm}
        assert not on_move.can_use_condition(ctx), \
            "Should NOT trigger when Ukkomon is not on the field"

    def test_condition_requires_your_turn(self, debug_runner):
        """Condition should only pass on owner's turn."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [UKKOMON])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]

        # Place a Digimon that "moved" from breeding
        moved_perm = runner.place_on_field(1, [AGUMON])
        ctx = {'moved_permanent': moved_perm}

        # P1's turn - should pass
        assert on_move.can_use_condition(ctx), "Should pass on owner's turn"

        # Switch to P2's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert not on_move.can_use_condition(ctx), "Should fail on opponent's turn"

    def test_condition_requires_own_digimon_move(self, debug_runner):
        """Condition requires that the moved permanent is a Digimon
        belonging to the effect owner (on the owner's battle area)."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [UKKOMON])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_move = [e for e in effects if e.timing == EffectTiming.OnMove][0]

        # Own Digimon move - should trigger
        own_moved = runner.place_on_field(1, [AGUMON])
        ctx = {'moved_permanent': own_moved}
        assert on_move.can_use_condition(ctx), \
            "Should trigger when own Digimon moves"

    # ── Integration: Reveal and select ────────────────────────────────

    def test_reveal_adds_digimon_to_hand(self, debug_runner):
        """Moving from breeding should trigger reveal-and-select.
        A Digimon card among revealed should be selectable to add to hand."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)

        # Setup: Ukkomon on P1 field, Lv.3 Digimon in breeding (ready to move)
        runner.place_on_field(1, [UKKOMON])
        runner.place_in_breeding(1, [AGUMON])  # Lv.3, can move

        # Stack library top: Digimon, Tamer, Option
        runner.clear_zone(1, "library")
        runner.inject_card(1, AGUMON, "library_top")       # pos 0 = Digimon
        runner.inject_card(1, TAI, "library_top")           # pos 0 now = Tamer
        runner.inject_card(1, SHADOW_WING, "library_top")   # pos 0 now = Option
        # After injections: library = [SHADOW_WING, TAI, AGUMON, ...]
        # Wait actually: library_top inserts at index 0 each time
        # So final order is: [SHADOW_WING, TAI, AGUMON]
        # Top 3 revealed will be SHADOW_WING (Option), TAI (Tamer), AGUMON (Digimon)
        # Filter: Digimon or Tamer => TAI and AGUMON are valid picks

        hand_before = len(runner.game.player1.hand_cards)
        lib_before = len(runner.game.player1.library_cards)

        # Move to Breeding phase and execute move
        runner.set_phase("Breeding")
        move_action = runner.find_action("Move")
        assert move_action is not None, "Should have Move action for Lv.3 in breeding"

        runner.execute(move_action)
        # After move, OnMove triggers Ukkomon's reveal effect
        # Auto-resolve: first picks from valid revealed cards, then handles hatch choice
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        lib_after = len(runner.game.player1.library_cards)

        # 1 card should have been added to hand from revealed
        assert hand_after == hand_before + 1, \
            f"Should add 1 card to hand from revealed. Before={hand_before}, After={hand_after}"

        # 2 remaining revealed cards go to deck bottom, so net library change = -3 +2 = -1
        # (but 3 were revealed and removed, 2 returned, 1 went to hand)
        assert lib_after == lib_before - 1, \
            f"Library should shrink by 1 (3 revealed, 1 to hand, 2 to bottom). Before={lib_before}, After={lib_after}"

    def test_reveal_no_qualifying_cards(self, debug_runner):
        """When no Digimon/Tamer among revealed, all go to deck bottom."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, [UKKOMON])
        runner.place_in_breeding(1, [AGUMON])

        # Stack library with only Option cards
        runner.clear_zone(1, "library")
        runner.inject_card(1, SHADOW_WING, "library_top")
        runner.inject_card(1, SHADOW_WING, "library_top")
        runner.inject_card(1, SHADOW_WING, "library_top")

        hand_before = len(runner.game.player1.hand_cards)
        lib_before = len(runner.game.player1.library_cards)

        runner.set_phase("Breeding")
        move_action = runner.find_action("Move")
        assert move_action is not None
        runner.execute(move_action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        lib_after = len(runner.game.player1.library_cards)

        # No qualifying cards => no card added to hand
        assert hand_after == hand_before, \
            f"Should NOT add any card to hand. Before={hand_before}, After={hand_after}"
        # All 3 returned to bottom
        assert lib_after == lib_before, \
            f"Library should stay same size. Before={lib_before}, After={lib_after}"

    def test_reveal_tamer_is_selectable(self, debug_runner):
        """Tamer cards should be valid picks from the revealed cards."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, [UKKOMON])
        runner.place_in_breeding(1, [AGUMON])

        # Only tamer among 3 cards
        runner.clear_zone(1, "library")
        runner.inject_card(1, SHADOW_WING, "library_top")
        runner.inject_card(1, TAI, "library_top")
        runner.inject_card(1, SHADOW_WING, "library_top")
        # Library: [SHADOW_WING, TAI, SHADOW_WING]
        # Only TAI is valid

        hand_before = len(runner.game.player1.hand_cards)

        runner.set_phase("Breeding")
        move_action = runner.find_action("Move")
        assert move_action is not None
        runner.execute(move_action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)

        # Exactly 1 card (the Tamer) should go to hand
        assert hand_after == hand_before + 1, \
            f"Should add 1 Tamer to hand. Before={hand_before}, After={hand_after}"

        # The added card should be a Tamer
        added_card = runner.game.player1.hand_cards[-1]
        assert added_card.is_tamer, "The added card should be a Tamer"

    # ── Integration: Optional hatch ───────────────────────────────────

    def test_hatch_offered_when_breeding_empty_and_digitama_available(self, debug_runner):
        """After reveal, if breeding is empty and digitama available, hatch choice appears."""
        # Use digi-eggs in deck
        digi_eggs = [YOKOMON] * 5
        main_deck = [UKKOMON] * 4 + [AGUMON] * 46
        runner = debug_runner(deck1=main_deck + digi_eggs, deck2=_FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, [UKKOMON])
        runner.place_in_breeding(1, [AGUMON])

        # Ensure digitama deck has eggs
        assert len(runner.game.player1.digitama_library_cards) > 0, \
            "Need digi-eggs for hatch test"

        # Stack library
        runner.clear_zone(1, "library")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        runner.set_phase("Breeding")
        move_action = runner.find_action("Move")
        assert move_action is not None
        runner.execute(move_action)

        # After move, breeding is now empty (we just moved out)
        # auto_resolve will pick first option at each step
        # The hatch choice should appear and auto_resolve picks "Yes, hatch"
        runner.auto_resolve()

        # After auto_resolve: hatch should have occurred
        snap = runner.snapshot()
        assert snap.p1_breeding is not None, \
            "Should have hatched a new egg in breeding area"

    def test_no_hatch_when_breeding_occupied(self, debug_runner):
        """If breeding area is already occupied, hatch choice should NOT appear."""
        digi_eggs = [YOKOMON] * 5
        main_deck = [UKKOMON] * 4 + [AGUMON] * 46
        runner = debug_runner(deck1=main_deck + digi_eggs, deck2=_FILLER_DECK, initial_memory=5)

        # Ukkomon on field, Agumon in breeding (to move), PLUS another Digimon
        # We'll manually occupy the breeding area after the move
        runner.place_on_field(1, [UKKOMON])
        runner.place_in_breeding(1, [AGUMON])

        runner.clear_zone(1, "library")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        # Clear digitama deck so hatch cannot happen
        runner.game.player1.digitama_library_cards.clear()

        runner.set_phase("Breeding")
        move_action = runner.find_action("Move")
        assert move_action is not None
        runner.execute(move_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # No digitama = no hatch
        assert snap.p1_breeding is None, \
            "Should NOT hatch when digitama deck is empty"

    def test_no_hatch_when_digitama_empty(self, debug_runner):
        """If digitama deck is empty, hatch choice should NOT appear."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, [UKKOMON])
        runner.place_in_breeding(1, [AGUMON])

        # Clear digitama deck
        runner.game.player1.digitama_library_cards.clear()

        runner.clear_zone(1, "library")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        runner.set_phase("Breeding")
        move_action = runner.find_action("Move")
        assert move_action is not None
        runner.execute(move_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert snap.p1_breeding is None, \
            "Should NOT hatch when digitama deck is empty"

    # ── Integration: Does NOT trigger on opponent's turn ──────────────

    def test_no_trigger_on_opponents_turn(self, debug_runner):
        """OnMove should NOT trigger when it is the opponent's turn."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)

        # Ukkomon on P1 field
        runner.place_on_field(1, [UKKOMON])
        # P2 has Digimon in breeding
        runner.place_in_breeding(2, [AGUMON])

        runner.clear_zone(1, "library")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        hand_before = len(runner.game.player1.hand_cards)

        # Switch to P2's turn
        game = runner.game
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.active_player = game.player2

        runner.set_phase("Breeding")
        move_action = runner.find_action("Move")
        assert move_action is not None, "P2 should be able to move from breeding"
        runner.execute(move_action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, \
            "Ukkomon should NOT trigger on opponent's turn"

    # ── Integration: effect does not fire from non-field ──────────────

    def test_no_trigger_from_hand(self, debug_runner):
        """OnMove should NOT trigger if Ukkomon is in hand (not on field)."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5)

        # Ukkomon in hand only
        runner.inject_card(1, UKKOMON, "hand")
        # Digimon in breeding
        runner.place_in_breeding(1, [AGUMON])

        runner.clear_zone(1, "library")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        hand_before = len(runner.game.player1.hand_cards)
        lib_before = len(runner.game.player1.library_cards)

        runner.set_phase("Breeding")
        move_action = runner.find_action("Move")
        assert move_action is not None
        runner.execute(move_action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        lib_after = len(runner.game.player1.library_cards)

        # No change to hand (Ukkomon stays in hand, no reveal triggered)
        assert hand_after == hand_before, \
            "Ukkomon in hand should NOT trigger OnMove reveal"
        assert lib_after == lib_before, \
            "Library should be unchanged when Ukkomon not on field"
