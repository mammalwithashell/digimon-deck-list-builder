"""Behavioral tests for BT24-083 Hiroko Sagisaka (Tamer).

Card text:
[Start of Your Turn] If you have 4 or less memory, by returning this Tamer to the
bottom of the deck, you may play 1 [Hiroko Sagisaka] or 1 [TS] trait Digimon card
with 5000 DP or less from your hand without paying the cost.
[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [TS] trait among
them to the hand. Return the rest to the bottom of the deck.
[Security] Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming

# BT24-031 Elecmon: Lv.3, 1000 DP, TS trait — valid play target for Start of Turn
HIROKO_DECK = ["BT24-083"] * 4 + ["BT24-031"] * 46


@pytest.mark.behavioral
class TestBT24083HirokoSagisaka:
    """Tests for BT24-083 Hiroko Sagisaka."""

    # ── On Play: Reveal top 3, add 1 [TS] to hand ────────────────────

    def test_on_play_reveal_adds_ts_card_to_hand(self, debug_runner):
        """On Play should reveal top 3 and add 1 TS-trait card to hand."""
        runner = debug_runner(deck1=HIROKO_DECK, deck2=HIROKO_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Inject TS cards at top of deck so reveal finds them
        runner.inject_card(1, "BT24-031", "library_top")
        runner.inject_card(1, "BT24-031", "library_top")
        runner.inject_card(1, "BT24-031", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        lib_before = snap_before.p1_library_size

        action = runner.find_action("Play Hiroko Sagisaka")
        assert action is not None, "Should be able to play Hiroko Sagisaka"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Tamer should be on field
        assert any(s.card_id == "BT24-083" for s in snap.p1_field), \
            "Hiroko should be on the field"
        # Hand: played Hiroko (-1), added 1 TS from reveal (+1) = net 0
        assert len(snap.p1_hand) == hand_before, \
            f"Expected hand size {hand_before} (play -1, reveal +1), got {len(snap.p1_hand)}"
        # Library: revealed 3 (1 to hand, 2 back to bottom) = net -1
        assert snap.p1_library_size == lib_before - 1, \
            f"Library should lose 1 card (to hand), got {snap.p1_library_size} vs expected {lib_before - 1}"

    def test_on_play_no_ts_in_reveal_no_crash(self, debug_runner):
        """On Play with no TS cards in top 3 should still play without error."""
        # Use a deck with non-TS cards at top
        runner = debug_runner(
            deck1=["BT24-083"] * 4 + ["ST1-03"] * 46,  # ST1-03 has no TS trait
            deck2=HIROKO_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        action = runner.find_action("Play Hiroko Sagisaka")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT24-083" for s in snap.p1_field), \
            "Hiroko should be on the field even if no TS cards revealed"

    # ── Security: Play this card without paying the cost ──────────────

    def test_security_plays_tamer_on_field(self, debug_runner):
        """When revealed from security, Hiroko should be played onto the field."""
        runner = debug_runner(deck1=HIROKO_DECK, deck2=HIROKO_DECK, initial_memory=10)
        runner.set_phase("Main")

        game = runner.game

        # Clear P2 security and place Hiroko as the only security card
        game.player2.security_cards.clear()
        runner.inject_card(2, "BT24-083", "security_top")

        # Place a strong attacker on P1's field (no summoning sickness)
        runner.place_on_field(1, ["ST1-10"], turn_played=-1)  # Phoenixmon 12000 DP

        before = runner.snapshot()
        p2_field_before = len(before.p2_field)

        action = runner.find_action("Attack player with Phoenixmon")
        assert action is not None, "Should find attack-player action for Phoenixmon"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Hiroko should be on P2's field (played from security)
        assert any(s.card_id == "BT24-083" for s in snap.p2_field), \
            "Hiroko should be played onto P2's field from security"
        # She should NOT be in P2's trash
        assert "BT24-083" not in snap.p2_trash, \
            "Hiroko should not be trashed — she was played from security"

    # ── Start of Turn: return to deck, play from hand ─────────────────

    def test_start_of_turn_return_and_play_ts_digimon(self, debug_runner):
        """Start of Turn with <=4 memory: return Hiroko, play TS Digimon from hand."""
        runner = debug_runner(deck1=HIROKO_DECK, deck2=HIROKO_DECK,
                              initial_memory=3, skip_shuffle=True)

        game = runner.game
        # Place Hiroko on P1's field
        runner.place_on_field(1, ["BT24-083"], turn_played=-1)
        # Clear hand so only the injected card is available for selection
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT24-031", "hand")

        before = runner.snapshot()
        assert any(s.card_id == "BT24-083" for s in before.p1_field), \
            "Hiroko should be on field before start-of-turn"

        # Trigger start-of-turn effects — Hiroko returns to deck, play selection opens
        game.execute_effects(EffectTiming.OnStartTurn)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Hiroko should no longer be on field (returned to deck bottom)
        assert not any(s.card_id == "BT24-083" for s in snap.p1_field), \
            "Hiroko should be returned to deck bottom"
        # BT24-031 Elecmon should be on field (played from hand for free)
        assert any(s.card_id == "BT24-031" for s in snap.p1_field), \
            "Elecmon (TS, DP<=5000) should be played from hand"
