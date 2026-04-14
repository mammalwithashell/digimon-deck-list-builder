"""Behavioral tests for BT24-031 Elecmon (Lv.3 Yellow Digimon).

Card text:
Alt-digi: Lv.2 with [TS] trait for cost 0
[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Iliad]
trait and 1 card with [TS] trait among them to the hand. Return the rest to
the bottom of the deck.
Inherited: [When Attacking] [Once Per Turn] You may add your top security card
to the hand. Then, if you have 0 security cards, <Recovery +1 (Deck)>.
"""

import pytest

# Minimal deck of BT24-031 (has traits: Mammal, Iliad, TS)
ELECMON_DECK = ["BT24-031"] * 50


@pytest.mark.behavioral
class TestBT24031Elecmon:
    """Tests for BT24-031 Elecmon."""

    # ── On Play: Reveal 3, add 1 Iliad + 1 TS ──────────────────────

    def test_on_play_reveal_adds_iliad_and_ts(self, debug_runner):
        """On Play should reveal top 3 and allow adding 1 Iliad + 1 TS card."""
        runner = debug_runner(deck1=ELECMON_DECK, deck2=ELECMON_DECK, initial_memory=10)

        runner.set_phase("Main")
        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Elecmon")
        assert action is not None, "Should be able to play Elecmon"
        runner.execute(action)
        # Resolve the two reveal selection passes
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT24-031" for s in snap.p1_field), \
            "Elecmon should be on the field after playing"
        # Hand: started with N, played 1 (-1), gained 2 from reveal (+2) = net +1
        assert len(snap.p1_hand) == hand_before + 1, \
            f"Expected hand size {hand_before + 1}, got {len(snap.p1_hand)}"

    def test_on_play_no_deck_no_crash(self, debug_runner):
        """On Play with empty deck should not crash."""
        runner = debug_runner(deck1=ELECMON_DECK, deck2=ELECMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-031", "hand")
        runner.clear_zone(1, "library")

        runner.set_phase("Main")
        action = runner.find_action("Play Elecmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT24-031" for s in snap.p1_field)

    # ── Inherited: When Attacking, may add security to hand ─────────

    def test_inherited_not_optional_at_effect_level(self, debug_runner):
        """The inherited effect should NOT be marked is_optional (optionality
        is inside the process via branch choice)."""
        runner = debug_runner(deck1=ELECMON_DECK, deck2=ELECMON_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["BT24-031", "BT24-031"])
        # BT24-031 is at bottom of stack (index 0) — that's where inherited effects come from
        card = perm.card_sources[0]
        effects = card.effect_list(None)

        inherited_effects = [e for e in effects if e.is_inherited_effect]
        assert len(inherited_effects) >= 1, "Should have inherited effect"
        for eff in inherited_effects:
            assert not eff.is_optional, \
                "Inherited When Attacking effect should not be is_optional"

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited effect should have Once Per Turn limit."""
        runner = debug_runner(deck1=ELECMON_DECK, deck2=ELECMON_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["BT24-031", "BT24-031"])
        card = perm.card_sources[0]
        effects = card.effect_list(None)

        inherited_effects = [e for e in effects if e.is_inherited_effect]
        assert len(inherited_effects) >= 1, "Should have inherited effect"
        eff = inherited_effects[0]
        assert eff.max_count_per_turn == 1, \
            "Inherited When Attacking should be Once Per Turn"

    def test_inherited_recovery_at_zero_security(self, debug_runner):
        """If player has 0 security after the choice, Recovery +1 should trigger."""
        runner = debug_runner(deck1=ELECMON_DECK, deck2=ELECMON_DECK, initial_memory=10)

        # Place a stack with BT24-031 inherited, no security
        perm = runner.place_on_field(1, ["BT24-031", "BT24-031"])
        runner.clear_zone(1, "security")
        runner.inject_card(1, "BT24-031", "library_top")

        snap_before = runner.snapshot()
        assert snap_before.p1_security_count == 0

        runner.set_phase("Main")
        attack = runner.find_action("Attack")
        if attack is not None:
            runner.execute(attack)
            runner.auto_resolve()

            snap = runner.snapshot()
            # Recovery +1 should fire since security was 0
            assert snap.p1_security_count == 1, \
                f"Recovery +1 should trigger at 0 security, got {snap.p1_security_count}"

    # ── Alt-Digi ────────────────────────────────────────────────────

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi should allow digivolving from Lv.2 with [TS] for cost 0."""
        runner = debug_runner(deck1=ELECMON_DECK, deck2=ELECMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-031", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost')]
        assert len(alt_digi) == 1, "Should have 1 alt-digi effect"
        eff = alt_digi[0]
        assert eff._alt_digi_cost == 0
        assert eff._alt_digi_level == 2
        assert eff._alt_digi_trait == "TS"
