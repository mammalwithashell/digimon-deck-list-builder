"""Behavioral tests for BT4-097 Kari Kamiya (Tamer, Purple, Cost 3).

Card text:
[All Turns] When a card is removed from your security stack,
by suspending this Tamer, gain 1 memory.
Security Effect [Security] Play this card without paying the cost.
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming

# Card references:
# ST1-03 Agumon: Red, Lv3, DP 2000 (filler)
# ST1-11 WarGreymon: Red, Lv6, DP 12000 (attacker for security checks)
FILLER = "ST1-03"
ATTACKER = "ST1-11"

# Deck with BT4-097 and filler
KARI_DECK = ["BT4-097"] * 4 + [FILLER] * 46


@pytest.mark.behavioral
class TestBT4097KariKamiya:
    """Tests for BT4-097 Kari Kamiya."""

    # -- [All Turns] OnLoseSecurity: suspend to gain 1 memory --

    def test_gains_memory_when_own_security_removed(self, debug_runner):
        """When Kari's owner loses security, suspending Kari should gain 1 memory."""
        runner = debug_runner(deck1=KARI_DECK, deck2=KARI_DECK, initial_memory=3)
        game = runner.game

        # Place Kari on P1 field (unsuspended)
        kari_perm = runner.place_on_field(1, ["BT4-097"])
        assert not kari_perm.is_suspended, "Kari should start unsuspended"

        memory_before = game.memory

        # Fire OnLoseSecurity for P1 (Kari's owner)
        game.player1._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": None, "player": game.player1},
        )
        runner.auto_resolve()

        # Kari should be suspended (cost)
        assert kari_perm.is_suspended, (
            "Kari should be suspended after gaining memory"
        )
        # Memory should have increased by 1 for P1
        assert game.memory == memory_before + 1, (
            f"Memory should increase by 1. Was {memory_before}, now {game.memory}"
        )

    def test_does_not_trigger_on_opponent_security_loss(self, debug_runner):
        """Kari should NOT trigger when opponent loses security (only own security)."""
        runner = debug_runner(deck1=KARI_DECK, deck2=KARI_DECK, initial_memory=3)
        game = runner.game

        kari_perm = runner.place_on_field(1, ["BT4-097"])
        memory_before = game.memory

        # Fire OnLoseSecurity for P2 (NOT Kari's owner)
        game.player2._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": None, "player": game.player2},
        )
        runner.auto_resolve()

        # Kari should NOT activate
        assert not kari_perm.is_suspended, (
            "Kari should NOT suspend when opponent loses security"
        )
        assert game.memory == memory_before, (
            f"Memory should not change. Was {memory_before}, now {game.memory}"
        )

    def test_does_not_trigger_when_already_suspended(self, debug_runner):
        """Kari should NOT trigger if already suspended (suspend is cost)."""
        runner = debug_runner(deck1=KARI_DECK, deck2=KARI_DECK, initial_memory=3)
        game = runner.game

        # Place Kari already suspended
        kari_perm = runner.place_on_field(1, ["BT4-097"], is_suspended=True)
        memory_before = game.memory

        # Fire OnLoseSecurity for P1
        game.player1._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": None, "player": game.player1},
        )
        runner.auto_resolve()

        # Memory should not change
        assert game.memory == memory_before, (
            f"Memory should not change when Kari is already suspended. "
            f"Was {memory_before}, now {game.memory}"
        )

    def test_all_turns_works_on_opponent_turn(self, debug_runner):
        """[All Turns] means Kari should trigger even on opponent's turn."""
        runner = debug_runner(deck1=KARI_DECK, deck2=KARI_DECK, initial_memory=3)
        game = runner.game

        kari_perm = runner.place_on_field(1, ["BT4-097"])

        # Switch turn to P2 (opponent's turn)
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        memory_before = game.memory

        # P1 loses security on opponent's turn
        game.player1._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": None, "player": game.player1},
        )
        runner.auto_resolve()

        # Kari should still activate (All Turns)
        assert kari_perm.is_suspended, (
            "Kari should activate on opponent's turn ([All Turns])"
        )
        # Memory gain: P1 gains 1 memory. P1 is opponent_player, so game.memory decreases by 1
        assert game.memory == memory_before - 1, (
            f"P1 gains 1 memory on opponent's turn (memory gauge moves toward P1). "
            f"Was {memory_before}, now {game.memory}"
        )

    def test_only_activates_once_per_security_loss(self, debug_runner):
        """Multiple security losses should not trigger if Kari is already suspended
        from the first trigger."""
        runner = debug_runner(deck1=KARI_DECK, deck2=KARI_DECK, initial_memory=3)
        game = runner.game

        kari_perm = runner.place_on_field(1, ["BT4-097"])
        memory_before = game.memory

        # First security loss - should trigger
        game.player1._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": None, "player": game.player1},
        )
        runner.auto_resolve()

        assert kari_perm.is_suspended
        memory_after_first = game.memory
        assert memory_after_first == memory_before + 1

        # Second security loss - should NOT trigger (already suspended)
        game.player1._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": None, "player": game.player1},
        )
        runner.auto_resolve()

        assert game.memory == memory_after_first, (
            f"Second trigger should not gain memory (already suspended). "
            f"Was {memory_after_first}, now {game.memory}"
        )

    def test_multiple_kari_on_field_each_triggers_independently(self, debug_runner):
        """Multiple Kari Kamiyas should each be able to trigger independently."""
        runner = debug_runner(deck1=KARI_DECK, deck2=KARI_DECK, initial_memory=3)
        game = runner.game

        kari1 = runner.place_on_field(1, ["BT4-097"])
        kari2 = runner.place_on_field(1, ["BT4-097"])
        memory_before = game.memory

        # Fire OnLoseSecurity for P1
        game.player1._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": None, "player": game.player1},
        )
        runner.auto_resolve()

        # Both Karis should suspend and gain 1 memory each (total +2)
        assert kari1.is_suspended, "First Kari should be suspended"
        assert kari2.is_suspended, "Second Kari should be suspended"
        assert game.memory == memory_before + 2, (
            f"Two Karis should gain 2 memory total. "
            f"Was {memory_before}, now {game.memory}"
        )

    def test_trash_security_card_also_triggers(self, debug_runner):
        """Using trash_security_card (non-attack removal) should also trigger Kari."""
        runner = debug_runner(deck1=KARI_DECK, deck2=KARI_DECK, initial_memory=3)
        game = runner.game

        kari_perm = runner.place_on_field(1, ["BT4-097"])

        # Ensure P1 has at least 1 security card
        assert len(game.player1.security_cards) > 0, "P1 needs security to trash"

        memory_before = game.memory
        sec_card = game.player1.security_cards[-1]

        # Use trash_security_card (which fires OnLoseSecurity internally)
        game.player1.trash_security_card(sec_card)
        runner.auto_resolve()

        assert kari_perm.is_suspended, (
            "Kari should trigger on trash_security_card"
        )
        assert game.memory == memory_before + 1, (
            f"Memory should increase by 1 after trash_security_card. "
            f"Was {memory_before}, now {game.memory}"
        )
