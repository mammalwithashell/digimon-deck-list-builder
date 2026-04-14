"""Behavioral tests for BT6-082 Sistermon Blanc (Lv.3 White Puppet).

Card text:
[All Turns] While you have a Digimon with [Huckmon] in its name or [Royal Knight]
    trait in play, all of your Digimon with [Sistermon] in their name gain <Blocker>.
[On Play] <Draw 1> (Draw 1 card from your deck.)

Inherited: None
"""

import pytest

# Minimal deck: 50 copies of BT6-082 for deterministic setup
BLANC_DECK = ["BT6-082"] * 50


@pytest.mark.behavioral
class TestBT6082SistermonBlanc:
    """Tests for BT6-082 Sistermon Blanc."""

    # ── Blocker Aura: Self ────────────────────────────────────────

    def test_blocker_with_huckmon_on_field(self, debug_runner):
        """Sistermon Blanc should have Blocker when a Huckmon is on the field."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        # Place Sistermon Blanc and Huckmon on P1's field
        runner.place_on_field(1, ["BT6-082"])
        runner.place_on_field(1, ["BT6-009"])  # Huckmon

        snap = runner.snapshot()
        blanc = [f for f in snap.p1_field if f.card_id == "BT6-082"][0]
        assert "blocker" in blanc.keywords, \
            f"Sistermon Blanc should have Blocker with Huckmon on field, got {blanc.keywords}"

    def test_blocker_with_royal_knight_on_field(self, debug_runner):
        """Sistermon Blanc should have Blocker when a Royal Knight is on the field."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-082"])
        runner.place_on_field(1, ["BT1-084"])  # Omnimon (Royal Knight trait)

        snap = runner.snapshot()
        blanc = [f for f in snap.p1_field if f.card_id == "BT6-082"][0]
        assert "blocker" in blanc.keywords, \
            f"Sistermon Blanc should have Blocker with Royal Knight on field, got {blanc.keywords}"

    def test_no_blocker_without_huckmon_or_rk(self, debug_runner):
        """Sistermon Blanc should NOT have Blocker without Huckmon or Royal Knight."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-082"])
        # No Huckmon or Royal Knight on field

        snap = runner.snapshot()
        blanc = [f for f in snap.p1_field if f.card_id == "BT6-082"][0]
        assert "blocker" not in blanc.keywords, \
            f"Sistermon Blanc should NOT have Blocker without Huckmon/RK, got {blanc.keywords}"

    # ── Blocker Aura: Cross-permanent ────────────────────────────

    def test_blocker_aura_grants_to_other_sistermon(self, debug_runner):
        """BT6-082's aura should grant Blocker to OTHER Sistermon on the field.

        Place BT6-084 (Sistermon Ciel, a Digimon with 'Sistermon' in name)
        alongside BT6-082 and a Huckmon. Sistermon Ciel should gain Blocker
        from BT6-082's aura.
        """
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-082"])  # Sistermon Blanc (host of aura)
        runner.place_on_field(1, ["BT6-084"])  # Sistermon Ciel (target)
        runner.place_on_field(1, ["BT6-009"])  # Huckmon (enabler)

        snap = runner.snapshot()
        ciel = [f for f in snap.p1_field if f.card_id == "BT6-084"][0]
        assert "blocker" in ciel.keywords, \
            f"Sistermon Ciel should gain Blocker from BT6-082's aura, got {ciel.keywords}"

    def test_blocker_aura_does_not_grant_to_non_sistermon(self, debug_runner):
        """BT6-082's aura should NOT grant Blocker to non-Sistermon Digimon."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-082"])  # Sistermon Blanc
        runner.place_on_field(1, ["BT6-009"])  # Huckmon (enabler, not a Sistermon)

        snap = runner.snapshot()
        huckmon = [f for f in snap.p1_field if f.card_id == "BT6-009"][0]
        assert "blocker" not in huckmon.keywords, \
            f"Huckmon should NOT gain Blocker from BT6-082's aura, got {huckmon.keywords}"

    def test_blocker_aura_not_active_without_enabler(self, debug_runner):
        """Without Huckmon/RK, even other Sistermon should NOT gain Blocker."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-082"])  # Sistermon Blanc
        runner.place_on_field(1, ["BT6-084"])  # Sistermon Ciel

        snap = runner.snapshot()
        ciel = [f for f in snap.p1_field if f.card_id == "BT6-084"][0]
        assert "blocker" not in ciel.keywords, \
            f"Sistermon Ciel should NOT have Blocker without enabler, got {ciel.keywords}"

    # ── Blocker Aura: Both turns ─────────────────────────────────

    def test_blocker_active_on_opponent_turn(self, debug_runner):
        """Blocker aura should be active on opponent's turn (All Turns)."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-082"])
        runner.place_on_field(1, ["BT6-009"])

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False

        snap = runner.snapshot()
        blanc = [f for f in snap.p1_field if f.card_id == "BT6-082"][0]
        assert "blocker" in blanc.keywords, \
            f"Blocker should be active on opponent's turn, got {blanc.keywords}"

    def test_blocker_active_on_own_turn(self, debug_runner):
        """Blocker aura should be active on own turn (All Turns)."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-082"])
        runner.place_on_field(1, ["BT6-009"])

        # Ensure own turn
        runner.game.player1.is_my_turn = True

        snap = runner.snapshot()
        blanc = [f for f in snap.p1_field if f.card_id == "BT6-082"][0]
        assert "blocker" in blanc.keywords, \
            f"Blocker should be active on own turn, got {blanc.keywords}"

    # ── On Play: Draw 1 ─────────────────────────────────────────

    def test_on_play_draw_1(self, debug_runner):
        """Playing Sistermon Blanc should draw 1 card."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        deck_before = snap_before.p1_library_size

        action = runner.find_action("Play Sistermon Blanc")
        assert action is not None, "Should be able to play Sistermon Blanc"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Hand: lost 1 (played), gained 1 (draw) = net 0
        # Deck: lost 1 (draw)
        assert snap.p1_library_size == deck_before - 1, \
            f"Deck should lose 1 card from draw, was {deck_before}, now {snap.p1_library_size}"

    # ── On Play condition: field check ───────────────────────────

    def test_on_play_effect_structure(self, debug_runner):
        """On Play Draw 1 effect should have proper field presence condition."""
        runner = debug_runner(deck1=BLANC_DECK, deck2=BLANC_DECK, initial_memory=5)

        runner.inject_card(1, "BT6-082", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        on_play_effects = [e for e in effects if getattr(e, 'is_on_play', False)]
        assert len(on_play_effects) == 1, \
            f"Should have exactly 1 On Play effect, got {len(on_play_effects)}"

        on_play = on_play_effects[0]
        # When in hand (not on field), the on_play condition should be False
        # because permanent_of_this_card() returns None
        result = on_play.can_use_condition({})
        assert result is False, \
            "On Play condition should be False when card is in hand (not on field)"
