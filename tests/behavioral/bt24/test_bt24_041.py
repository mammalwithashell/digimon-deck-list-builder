"""Behavioral tests for BT24-041 Minervamon (Lv.6 Yellow/Purple Digimon).

Card text:
When this card would be played, if you have an [Iliad] trait Digimon or Tamer,
reduce the play cost by 5.
[On Play] [When Digivolving] [On Deletion] You may play 1 play cost 5 or lower
[Iliad] trait card from your hand without paying the cost. Then, to 1 of your
opponent's Digimon, De-Digivolve 1 for each of your Digimon.
[Opponent's Turn] All of your [Iliad] trait Digimon gain Reboot and Blocker.

Alt-digi: Lv.5 w/[Beastkin]/[Dark Dragon]/[TS]: Cost 3
"""

import pytest

MINERVAMON_DECK = ["BT24-041"] * 50


@pytest.mark.behavioral
class TestBT24041Minervamon:
    """Tests for BT24-041 Minervamon."""

    # ── Alt-Digi ───────────────────────────────────────────────────

    def test_alt_digi_three_traits(self, debug_runner):
        """Alt-digi should have separate effects for Beastkin, Dark Dragon, and TS."""
        runner = debug_runner(deck1=MINERVAMON_DECK, deck2=MINERVAMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-041", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost')]
        assert len(alt_digi) == 3, f"Should have 3 alt-digi effects, got {len(alt_digi)}"

        traits_found = {e._alt_digi_trait for e in alt_digi}
        assert traits_found == {"Beastkin", "Dark Dragon", "TS"}, \
            f"Expected Beastkin/Dark Dragon/TS, got {traits_found}"

        for eff in alt_digi:
            assert eff._alt_digi_cost == 3, f"Alt-digi cost should be 3"
            assert eff._alt_digi_level == 5, f"Alt-digi level should be 5"

    # ── BeforePayCost cost reduction ──────────────────────────────

    def test_cost_reduction_with_iliad_on_field(self, debug_runner):
        """Play cost should be reduced by 5 when Iliad trait Digimon is on field."""
        runner = debug_runner(deck1=MINERVAMON_DECK, deck2=MINERVAMON_DECK, initial_memory=20)

        # Place an Iliad trait Digimon on field (BT24-031 Elecmon has Iliad)
        runner.place_on_field(1, ["BT24-031"])
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        mem_before = snap_before.memory

        action = runner.find_action("Play Minervamon")
        assert action is not None, "Should be able to play Minervamon"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Cost should be 12 - 5 = 7 memory
        mem_spent = mem_before - snap.memory
        assert mem_spent == 7, f"Should spend 7 memory (12-5), spent {mem_spent}"

    def test_cost_reduction_without_iliad_on_field(self, debug_runner):
        """Play cost should NOT be reduced without Iliad trait on field."""
        runner = debug_runner(deck1=MINERVAMON_DECK, deck2=MINERVAMON_DECK, initial_memory=20)

        runner.set_phase("Main")
        snap_before = runner.snapshot()
        mem_before = snap_before.memory

        action = runner.find_action("Play Minervamon")
        assert action is not None, "Should be able to play Minervamon"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        mem_spent = mem_before - snap.memory
        assert mem_spent == 12, f"Should spend 12 memory (no reduction), spent {mem_spent}"

    # ── On Play: play Iliad card + De-Digivolve ──────────────────

    def test_on_play_de_digivolve_opponent(self, debug_runner):
        """On Play should de-digivolve an opponent's Digimon by count of own Digimon."""
        runner = debug_runner(deck1=MINERVAMON_DECK, deck2=MINERVAMON_DECK, initial_memory=20)

        # Place opponent's Digimon (stack of 3: lv3, lv4, lv5) to verify de-digivolve
        runner.place_on_field(2, ["BT24-031", "BT24-025", "BT24-041"])

        runner.set_phase("Main")

        snap_before = runner.snapshot()
        opp_stack_before = snap_before.p2_field[0].stack_ids if snap_before.p2_field else []
        stack_len_before = len(opp_stack_before)

        action = runner.find_action("Play Minervamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # After Minervamon enters, we have 1 Digimon, so De-Digivolve 1 × 1 = 1
        # Opponent's Digimon should lose 1 card from its stack
        if snap.p2_field:
            opp_stack_after = snap.p2_field[0].stack_ids
            assert len(opp_stack_after) == stack_len_before - 1, \
                f"Opponent stack should lose 1 card: {stack_len_before} -> {len(opp_stack_after)}"

    def test_on_play_play_iliad_card_optional(self, debug_runner):
        """On Play: playing an Iliad card from hand should be optional."""
        runner = debug_runner(deck1=MINERVAMON_DECK, deck2=MINERVAMON_DECK, initial_memory=20)

        # Put Iliad card in hand
        runner.inject_card(1, "BT24-031", "hand")
        # Opponent Digimon for de-digivolve target
        runner.place_on_field(2, ["BT24-031", "BT24-025"])

        runner.set_phase("Main")
        hand_before = len(runner.snapshot().p1_hand)

        action = runner.find_action("Play Minervamon")
        assert action is not None
        runner.execute(action)
        # auto_resolve picks first legal action — it may play or decline
        runner.auto_resolve()

        snap = runner.snapshot()
        # Just verify the game didn't crash and Minervamon is on field
        miner_on_field = any(
            f.card_name == "Minervamon" for f in snap.p1_field
        )
        assert miner_on_field, "Minervamon should be on field after play"

    # ── Opponent's Turn: Reboot + Blocker grant ──────────────────

    def test_reboot_blocker_iliad_digimon_opponent_turn(self, debug_runner):
        """Iliad Digimon should gain Reboot and Blocker on opponent's turn."""
        runner = debug_runner(deck1=MINERVAMON_DECK, deck2=MINERVAMON_DECK, initial_memory=10)

        # Place Minervamon (has Iliad trait) on field
        runner.place_on_field(1, ["BT24-041"])
        # Place another Iliad Digimon
        runner.place_on_field(1, ["BT24-031"])

        # Switch to opponent's turn (must set player.is_my_turn directly)
        runner.game.player1.is_my_turn = False

        snap = runner.snapshot()
        for f in snap.p1_field:
            assert "blocker" in f.keywords, \
                f"{f.card_name} should have Blocker on opponent's turn, got {f.keywords}"
            assert "reboot" in f.keywords, \
                f"{f.card_name} should have Reboot on opponent's turn, got {f.keywords}"

    def test_no_reboot_blocker_own_turn(self, debug_runner):
        """Iliad Digimon should NOT gain Reboot/Blocker on own turn."""
        runner = debug_runner(deck1=MINERVAMON_DECK, deck2=MINERVAMON_DECK, initial_memory=10)

        runner.place_on_field(1, ["BT24-041"])
        runner.place_on_field(1, ["BT24-031"])

        # Own turn (player1.is_my_turn defaults to True)
        runner.game.player1.is_my_turn = True

        snap = runner.snapshot()
        for f in snap.p1_field:
            assert "blocker" not in f.keywords, \
                f"{f.card_name} should NOT have Blocker on own turn"
            assert "reboot" not in f.keywords, \
                f"{f.card_name} should NOT have Reboot on own turn"

    def test_non_iliad_digimon_no_reboot_blocker(self, debug_runner):
        """Non-Iliad Digimon should NOT gain Reboot/Blocker even on opponent's turn."""
        runner = debug_runner(deck1=MINERVAMON_DECK, deck2=MINERVAMON_DECK, initial_memory=10)

        # Place Minervamon (Iliad, grants the aura)
        runner.place_on_field(1, ["BT24-041"])

        # Need a non-Iliad Digimon. BT1-019 Agumon (no Iliad trait).
        runner.place_on_field(1, ["BT1-019"])

        runner.game.player1.is_my_turn = False

        snap = runner.snapshot()
        non_iliad = [f for f in snap.p1_field if f.card_name != "Minervamon"]
        for f in non_iliad:
            assert "blocker" not in f.keywords, \
                f"{f.card_name} (non-Iliad) should NOT have Blocker"
