"""Behavioral tests for BT24-034 Aegiomon (Lv.4 Yellow Digimon).

Card text:
＜Barrier＞
[When Moving] [On Play] [When Digivolving] By adding your top security card
to the hand, you may play 1 [TS] trait Tamer card from your hand without
paying the cost. This effect can't play cards with the same name as any of
your Tamers.
Inherited: ＜Barrier＞

Alt-digi: [Elecmon]/Lv.3 w/[TS] trait: Cost 2
"""

import pytest

# BT24-034 Aegiomon is Yellow Lv.4, traits: Shaman, Iliad, TS
AEGIOMON_DECK = ["BT24-034"] * 50


@pytest.mark.behavioral
class TestBT24034Aegiomon:
    """Tests for BT24-034 Aegiomon."""

    # ── Barrier ────────────────────────────────────────────────────

    def test_barrier_own_effect(self, debug_runner):
        """Own Barrier effect should be present."""
        runner = debug_runner(deck1=AEGIOMON_DECK, deck2=AEGIOMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-034", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        barrier = [e for e in effects if getattr(e, '_is_barrier', False) and not e.is_inherited_effect]
        assert len(barrier) == 1, "Should have 1 own Barrier effect"

    def test_barrier_inherited_effect(self, debug_runner):
        """Inherited Barrier effect should be present."""
        runner = debug_runner(deck1=AEGIOMON_DECK, deck2=AEGIOMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-034", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        barrier_inh = [e for e in effects if getattr(e, '_is_barrier', False) and e.is_inherited_effect]
        assert len(barrier_inh) == 1, "Should have 1 inherited Barrier effect"

    # ── Alt-Digi ───────────────────────────────────────────────────

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi: [Elecmon] or Lv.3 w/[TS], cost 2."""
        runner = debug_runner(deck1=AEGIOMON_DECK, deck2=AEGIOMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-034", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost')]
        assert len(alt_digi) == 1, "Should have 1 alt-digi effect"
        eff = alt_digi[0]
        assert eff._alt_digi_cost == 2
        assert eff._alt_digi_level == 3
        assert eff._alt_digi_name == "Elecmon"
        assert eff._alt_digi_trait == "TS"

    # ── On Play: security-to-hand + play TS Tamer ─────────────────

    def test_on_play_security_moves_to_hand(self, debug_runner):
        """On Play should move top security to hand (mandatory cost)."""
        runner = debug_runner(deck1=AEGIOMON_DECK, deck2=AEGIOMON_DECK, initial_memory=10)

        runner.set_phase("Main")
        snap_before = runner.snapshot()
        sec_before = snap_before.p1_security_count
        assert sec_before > 0, "Need security for cost"
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Aegiomon")
        assert action is not None, "Should be able to play Aegiomon"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Security should decrease (cost paid)
        assert snap.p1_security_count < sec_before, \
            "Security should decrease from adding top card to hand"

    def test_on_play_tamer_play_is_optional(self, debug_runner):
        """After security cost, tamer play should be optional (canNoSelect: true).
        With no tamers in hand, effect should still activate and move security."""
        runner = debug_runner(deck1=AEGIOMON_DECK, deck2=AEGIOMON_DECK, initial_memory=10)

        runner.set_phase("Main")
        snap_before = runner.snapshot()
        sec_before = snap_before.p1_security_count

        action = runner.find_action("Play Aegiomon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Security should still decrease even without tamer targets
        assert snap.p1_security_count < sec_before, \
            "Security cost should be paid even when no tamers to play"

    # ── On Play: no security = no activation ──────────────────────

    def test_on_play_no_security_no_activation(self, debug_runner):
        """With 0 security, the effect should not activate."""
        runner = debug_runner(deck1=AEGIOMON_DECK, deck2=AEGIOMON_DECK, initial_memory=10)

        runner.clear_zone(1, "security")

        runner.set_phase("Main")
        snap_before = runner.snapshot()
        assert snap_before.p1_security_count == 0

        action = runner.find_action("Play Aegiomon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert snap.p1_security_count == 0, "No security change when starting at 0"
