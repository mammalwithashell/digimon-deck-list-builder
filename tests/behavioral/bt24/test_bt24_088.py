"""Behavioral tests for BT24-088 Asuna Shiroki (Tamer).

Card text:
[Start of Your Turn] If you have 4 or less memory, by returning this Tamer to the
bottom of the deck, you may play 1 [Asuna Shiroki] or 1 level 4 or lower Digimon
card with the [TS] trait or [Three Musketeers] in its text from your trash without
paying the cost.
[On Play] By trashing 1 card with [Three Musketeers] in its text or the [TS] trait
from your hand, <Draw 2>.
[Security] Play this card without paying the cost.
"""

import pytest

# BT24-031 Elecmon: Lv.3, traits include TS — valid hand-filter target
# BT24-088 Asuna Shiroki: Tamer, mentions "Three Musketeers" in text (no trait)
ASUNA_DECK = ["BT24-088"] * 4 + ["BT24-031"] * 46


@pytest.mark.behavioral
class TestBT24088AsunShiroki:
    """Tests for BT24-088 Asuna Shiroki."""

    # ── On Play: By trashing TS/Three Musketeers card, Draw 2 ──────

    def test_on_play_trash_ts_card_draw_2(self, debug_runner):
        """On Play should allow trashing a TS-trait card from hand to draw 2."""
        runner = debug_runner(deck1=ASUNA_DECK, deck2=ASUNA_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Ensure we have a TS card (BT24-031 Elecmon) in hand
        runner.inject_card(1, "BT24-031", "hand")
        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Asuna Shiroki")
        assert action is not None, "Should be able to play Asuna Shiroki"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Tamer should be on field
        assert any(s.card_id == "BT24-088" for s in snap.p1_field), \
            "Asuna should be on the field"
        # Hand: started with N, played Asuna (-1), trashed 1 TS card (-1), drew 2 (+2) = net 0
        assert len(snap.p1_hand) == hand_before, \
            f"Expected hand size {hand_before} (play -1, trash -1, draw +2), got {len(snap.p1_hand)}"

    def test_on_play_no_valid_hand_card_no_crash(self, debug_runner):
        """On Play with no valid trash targets should still play the tamer."""
        runner = debug_runner(
            deck1=["BT24-088"] * 4 + ["BT1-001"] * 46,
            deck2=ASUNA_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        action = runner.find_action("Play Asuna Shiroki")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT24-088" for s in snap.p1_field), \
            "Asuna should be on the field even without valid trash targets"

    # ── On Play: Asuna Shiroki itself has "Three Musketeers" in text ─

    def test_on_play_trash_three_musketeers_text_card(self, debug_runner):
        """On Play should accept cards with 'Three Musketeers' in their effect
        text, not just in traits. Asuna herself has it in text."""
        runner = debug_runner(deck1=ASUNA_DECK, deck2=ASUNA_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Inject a second Asuna into hand — she has "Three Musketeers" in text
        runner.inject_card(1, "BT24-088", "hand")
        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Asuna Shiroki")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT24-088" for s in snap.p1_field), \
            "Asuna should be on the field"
        # Hand: played 1 (-1), trashed 1 Asuna from hand (-1), drew 2 (+2) = net 0
        assert len(snap.p1_hand) == hand_before, \
            f"Expected hand size {hand_before} after trash+draw, got {len(snap.p1_hand)}"
