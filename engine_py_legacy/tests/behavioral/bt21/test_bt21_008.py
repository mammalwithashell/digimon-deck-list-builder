"""Behavioral tests for BT21-008 Elizamon (Lv.3 Red Digimon).

Card text:
[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Reptile] or
[Dragonkin] trait and 1 card with the [LIBERATOR] trait among them to the hand.
Return the rest to the bottom of the deck.

Inherited Effect:
[Your Turn] [Once Per Turn] When your opponent's security stack is removed from,
gain 1 memory.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT21008Elizamon:
    """Tests for BT21-008 Elizamon."""

    # ── Helper ──────────────────────────────────────────────────────

    @staticmethod
    def _fire_security_loss(runner, losing_player):
        """Simulate a security loss event for the given player.

        Directly fires OnLoseSecurity timing with the correct context,
        matching what Player.security_attack() does at the end.
        """
        runner.game.execute_effects(
            EffectTiming.OnLoseSecurity,
            {"player": losing_player},
        )

    # ── Effect 0: [On Play] Reveal top 3 ────────────────────────────

    def test_on_play_reveal_adds_reptile_and_liberator(self, debug_runner):
        """[On Play] reveals top 3, player selects 1 Reptile/Dragonkin and 1 LIBERATOR to hand."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Inject Elizamon into hand
        runner.inject_card(1, "BT21-008", "hand")

        # Stack deck: top-to-bottom order (inject in reverse)
        # BT21-007 = Agumon (Reptile, matches filter 0)
        # BT18-060 = Lv.3 Digimon (Unknown + LIBERATOR, matches filter 1)
        # ST1-16 = Gaia Force (Option, no traits, filler)
        runner.inject_card(1, "ST1-16", "library_top")
        runner.inject_card(1, "BT18-060", "library_top")
        runner.inject_card(1, "BT21-007", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        lib_before = snap_before.p1_library_size

        action = runner.find_action("Play Elizamon")
        assert action is not None, "Should be able to play Elizamon from hand"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Elizamon should be on field
        assert any(
            s.card_id == "BT21-008" for s in snap_after.p1_field
        ), "Elizamon should be on the field"

        # 2 cards added to hand (Reptile + LIBERATOR), Elizamon left hand
        # Net change: hand_before - 1 (played) + 2 (added) = hand_before + 1
        assert len(snap_after.p1_hand) == hand_before - 1 + 2, (
            f"Should add 2 cards to hand (1 Reptile + 1 LIBERATOR); "
            f"hand went from {hand_before} to {len(snap_after.p1_hand)}"
        )

        # Filler goes to deck bottom: library lost 3, returned 1
        assert snap_after.p1_library_size == lib_before - 2, (
            f"Library should lose 2 net cards (3 revealed, 1 returned to bottom); "
            f"went from {lib_before} to {snap_after.p1_library_size}"
        )

    def test_on_play_dragonkin_matches_first_filter(self, debug_runner):
        """[On Play] Dragonkin trait should match the first filter (Reptile/Dragonkin)."""
        runner = debug_runner(initial_memory=15)
        runner.set_phase("Main")

        runner.inject_card(1, "BT21-008", "hand")

        # Stack deck: BT1-025 (WarGreymon, Dragonkin), BT18-060 (LIBERATOR), ST1-16 (filler)
        runner.inject_card(1, "ST1-16", "library_top")
        runner.inject_card(1, "BT18-060", "library_top")
        runner.inject_card(1, "BT1-025", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Elizamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Should add 2 cards (Dragonkin + LIBERATOR)
        assert len(snap_after.p1_hand) == hand_before - 1 + 2, (
            f"Should add Dragonkin + LIBERATOR to hand; "
            f"hand went from {hand_before} to {len(snap_after.p1_hand)}"
        )

    def test_on_play_no_matches_skips_pass(self, debug_runner):
        """[On Play] If no cards match a filter, that pass is skipped."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "BT21-008", "hand")

        # Stack deck: all fillers (no Reptile/Dragonkin, no LIBERATOR)
        runner.inject_card(1, "ST1-16", "library_top")
        runner.inject_card(1, "ST1-16", "library_top")
        runner.inject_card(1, "ST1-16", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        lib_before = snap_before.p1_library_size

        action = runner.find_action("Play Elizamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # No cards match either filter, so no cards added
        assert len(snap_after.p1_hand) == hand_before - 1, (
            "Should add 0 cards when no matches; "
            f"hand went from {hand_before} to {len(snap_after.p1_hand)}"
        )
        # All 3 fillers return to deck bottom
        assert snap_after.p1_library_size == lib_before, (
            "All 3 revealed cards should return to deck bottom"
        )

    def test_on_play_card_matching_both_filters(self, debug_runner):
        """[On Play] A card with both Reptile+LIBERATOR can only be picked by one pass."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "BT21-008", "hand")

        # Stack deck with a card matching BOTH filters plus fillers
        # Another BT21-008 (Reptile + LIBERATOR) matches both
        runner.inject_card(1, "ST1-16", "library_top")
        runner.inject_card(1, "ST1-16", "library_top")
        runner.inject_card(1, "BT21-008", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Elizamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # The duplicate Elizamon is picked by pass 0 (Reptile), removed from pool
        # Pass 1 (LIBERATOR) finds no remaining matching cards
        # So only 1 card added to hand
        assert len(snap_after.p1_hand) == hand_before - 1 + 1, (
            f"Dual-trait card picked by one pass, other pass finds nothing; "
            f"hand went from {hand_before} to {len(snap_after.p1_hand)}"
        )

    # ── Effect 1: Inherited [Your Turn] [OPT] security removed ─────

    def test_inherited_gain_memory_on_opponent_security_loss(self, debug_runner):
        """Inherited: gain 1 memory when opponent's security is removed (your turn)."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        # Place a Digimon with Elizamon as inherited (in stack)
        perm = runner.place_on_field(1, ["BT21-008", "ST1-03"])

        mem_before = runner.snapshot().memory

        # Fire OnLoseSecurity with event_player = P2 (opponent lost security)
        self._fire_security_loss(runner, runner.game.player2)
        runner.auto_resolve()

        mem_after = runner.snapshot().memory
        assert mem_after == mem_before + 1, (
            f"Should gain 1 memory when opponent security removed; "
            f"memory went from {mem_before} to {mem_after}"
        )

    def test_inherited_no_trigger_on_own_security_loss(self, debug_runner):
        """CRITICAL: inherited must NOT trigger when OWN security is removed."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        # Place a Digimon with Elizamon inherited on P1's field
        perm = runner.place_on_field(1, ["BT21-008", "ST1-03"])

        mem_before = runner.snapshot().memory

        # Fire OnLoseSecurity with event_player = P1 (OWN security lost)
        self._fire_security_loss(runner, runner.game.player1)
        runner.auto_resolve()

        mem_after = runner.snapshot().memory
        assert mem_after == mem_before, (
            f"Should NOT gain memory when OWN security is removed; "
            f"memory went from {mem_before} to {mem_after}"
        )

    def test_inherited_no_trigger_on_opponent_turn(self, debug_runner):
        """Inherited: should NOT trigger on opponent's turn."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        # Place Digimon with Elizamon inherited on P1's field
        perm = runner.place_on_field(1, ["BT21-008", "ST1-03"])

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        mem_before = runner.snapshot().memory

        # Opponent security removed on opponent's turn
        self._fire_security_loss(runner, runner.game.player2)
        runner.auto_resolve()

        mem_after = runner.snapshot().memory
        assert mem_after == mem_before, (
            f"Should NOT gain memory on opponent's turn; "
            f"memory went from {mem_before} to {mem_after}"
        )

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited: [Once Per Turn] should only trigger once."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT21-008", "ST1-03"])

        mem_before = runner.snapshot().memory

        # First security removal
        self._fire_security_loss(runner, runner.game.player2)
        runner.auto_resolve()

        mem_after_first = runner.snapshot().memory
        assert mem_after_first == mem_before + 1, (
            "First security removal should gain 1 memory"
        )

        # Second security removal
        self._fire_security_loss(runner, runner.game.player2)
        runner.auto_resolve()

        mem_after_second = runner.snapshot().memory
        assert mem_after_second == mem_after_first, (
            f"Second security removal should NOT gain memory (OPT); "
            f"memory went from {mem_after_first} to {mem_after_second}"
        )

    def test_inherited_requires_field_presence(self, debug_runner):
        """Inherited: should NOT trigger if the card is not on the field."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        # Put Elizamon in hand only, NOT on field as inherited
        runner.inject_card(1, "BT21-008", "hand")

        mem_before = runner.snapshot().memory

        # Opponent security removed
        self._fire_security_loss(runner, runner.game.player2)
        runner.auto_resolve()

        mem_after = runner.snapshot().memory
        assert mem_after == mem_before, (
            "Should NOT gain memory when Elizamon is in hand (not inherited on field)"
        )
