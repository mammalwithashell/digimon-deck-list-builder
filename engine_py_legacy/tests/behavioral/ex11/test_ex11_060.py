"""Behavioral tests for EX11-060 Arisa Kinosaki (Tamer).

Card text:
  [Start of Your Turn] If you have 2 or less memory, set it to 3.
  [All Turns] When any of your Tokens or [Puppet] trait Digimon are deleted,
      by suspending this Tamer, <Draw 1>. If deleted by <Overclock>, you may
      play 1 level 4 or lower [Puppet] trait Digimon card from your hand
      without paying the cost.
  [Security] Play this card without paying the cost.

Key behaviors:
  1. Memory set to 3 fires at OnStartTurn (before draw), only on your turn
  2. Deletion observer: triggers on own Token or Puppet Digimon deletion
  3. Cost: suspend this Tamer (must be unsuspended)
  4. Always draws 1 on trigger
  5. If deleted by Overclock: also may play Puppet Lv.4- from hand free
  6. [All Turns] means fires on both players' turns
"""

import pytest

from engine_py_legacy.engine.runners.debug_runner import DebugRunner
from engine_py_legacy.engine.data.card_registry import CardRegistry
from engine_py_legacy.engine.data.enums import GamePhase, EffectTiming


# ---- Helpers ----

# A small deck list that includes EX11-060 and Puppet Digimon
TAMER_ID = "EX11-060"
PUPPET_LV4 = "EX11-053"   # Omekamon, Lv.4 Puppet
PUPPET_LV3 = "EX11-019"   # Shoemon, Lv.3 Puppet
NON_PUPPET = "BT1-019"    # A non-Puppet generic Digimon


def _build_deck_with(card_ids: list[str], filler: str = "BT1-019", count: int = 50) -> list[str]:
    """Build a test deck from specific card IDs, padded with filler."""
    deck = list(card_ids)
    while len(deck) < count:
        deck.append(filler)
    return deck


# ---- Test: Start of Turn Memory Effect ----

class TestStartOfTurnMemory:
    """[Start of Your Turn] If you have 2 or less memory, set it to 3."""

    def test_memory_set_to_3_when_2_or_less(self, debug_runner):
        """Memory at 2 should become 3 at start of turn."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=0)

        # Place tamer on P1's field
        runner.place_on_field(1, [TAMER_ID])
        runner.set_memory(2)

        # Advance to start of P1's turn
        # We need to end the current turn first, then the opponent's turn
        # Use set_phase to jump to start-of-turn processing
        game = runner.game
        # Simulate start of turn by calling phase_start_turn directly
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.memory = 2

        # Execute OnStartTurn effects
        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 3, f"Memory should be 3, got {game.memory}"

    def test_memory_not_changed_when_above_2(self, debug_runner):
        """Memory at 3 or more should stay unchanged."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=0)

        runner.place_on_field(1, [TAMER_ID])
        game = runner.game
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.memory = 5

        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 5, f"Memory should stay at 5, got {game.memory}"

    def test_memory_set_to_3_when_negative(self, debug_runner):
        """Memory at negative (opponent's side) should become 3."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=0)

        runner.place_on_field(1, [TAMER_ID])
        game = runner.game
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.memory = -3

        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 3, f"Memory should be 3, got {game.memory}"

    def test_memory_not_set_on_opponent_turn(self, debug_runner):
        """Should not fire during opponent's turn."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=0)

        runner.place_on_field(1, [TAMER_ID])
        game = runner.game
        # Set P2 as the turn player
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player2.is_my_turn = True
        game.player1.is_my_turn = False
        game.memory = 1

        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 1, f"Memory should stay at 1 on opponent turn, got {game.memory}"


# ---- Test: Deletion Observer - Draw 1 ----

class TestDeletionObserverDraw:
    """[All Turns] When Token/Puppet deleted, suspend tamer -> Draw 1."""

    def test_draw_on_puppet_deletion(self, debug_runner):
        """Deleting a Puppet Digimon should trigger Draw 1."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4, PUPPET_LV3])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=3)

        # Place tamer and puppet on P1's field
        runner.place_on_field(1, [TAMER_ID])
        puppet_perm = runner.place_on_field(1, [PUPPET_LV4])

        game = runner.game
        p1 = game.player1
        hand_before = len(p1.hand_cards)
        lib_before = len(p1.library_cards)

        # Verify tamer is not suspended
        tamer_perm = [p for p in p1.battle_area if p.is_tamer][0]
        assert not tamer_perm.is_suspended

        # Delete the puppet
        p1.delete_permanent(puppet_perm)

        # Tamer should now be suspended (cost)
        assert tamer_perm.is_suspended, "Tamer should be suspended after observer triggers"
        # Should have drawn 1
        assert len(p1.hand_cards) == hand_before + 1, \
            f"Should have drawn 1 card, hand was {hand_before}, now {len(p1.hand_cards)}"

    def test_draw_on_token_deletion(self, debug_runner):
        """Deleting a Token should trigger Draw 1."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=3)

        # Place tamer on P1's field
        runner.place_on_field(1, [TAMER_ID])

        game = runner.game
        p1 = game.player1

        # Create a token on P1's field
        game.effect_play_token(p1, 'familiar')
        token_perm = [p for p in p1.battle_area if getattr(p, 'is_token', False)][0]

        hand_before = len(p1.hand_cards)

        # Delete the token
        p1.delete_permanent(token_perm)

        tamer_perm = [p for p in p1.battle_area if p.is_tamer][0]
        assert tamer_perm.is_suspended, "Tamer should be suspended"
        assert len(p1.hand_cards) == hand_before + 1, "Should have drawn 1"

    def test_no_trigger_when_tamer_already_suspended(self, debug_runner):
        """Should not trigger if tamer is already suspended (can't pay cost)."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=3)

        # Place suspended tamer and puppet
        runner.place_on_field(1, [TAMER_ID], is_suspended=True)
        puppet_perm = runner.place_on_field(1, [PUPPET_LV4])

        game = runner.game
        p1 = game.player1
        hand_before = len(p1.hand_cards)

        p1.delete_permanent(puppet_perm)

        # Should NOT have drawn
        assert len(p1.hand_cards) == hand_before, "Should not draw when tamer is suspended"

    def test_no_trigger_on_non_puppet_non_token_deletion(self, debug_runner):
        """Should not trigger when a non-Puppet, non-Token Digimon is deleted."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, NON_PUPPET, NON_PUPPET])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=3)

        runner.place_on_field(1, [TAMER_ID])
        non_puppet_perm = runner.place_on_field(1, [NON_PUPPET])

        game = runner.game
        p1 = game.player1
        hand_before = len(p1.hand_cards)

        p1.delete_permanent(non_puppet_perm)

        tamer_perm = [p for p in p1.battle_area if p.is_tamer][0]
        assert not tamer_perm.is_suspended, "Tamer should NOT be suspended"
        assert len(p1.hand_cards) == hand_before, "Should NOT draw"

    def test_triggers_on_opponent_turn(self, debug_runner):
        """[All Turns] means it should also trigger on opponent's turn."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=3)

        runner.place_on_field(1, [TAMER_ID])
        puppet_perm = runner.place_on_field(1, [PUPPET_LV4])

        game = runner.game
        p1 = game.player1
        # Switch turn to opponent
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player2.is_my_turn = True
        game.player1.is_my_turn = False

        hand_before = len(p1.hand_cards)

        # Opponent's effect deletes P1's puppet
        p1.delete_permanent(puppet_perm)

        tamer_perm = [p for p in p1.battle_area if p.is_tamer][0]
        assert tamer_perm.is_suspended, "Tamer should suspend on opponent turn too"
        assert len(p1.hand_cards) == hand_before + 1, "Should draw on opponent turn"


# ---- Test: Overclock Deletion - Play Puppet Lv.4 from Hand ----

class TestOverclockDeletion:
    """If deleted by Overclock, may play Puppet Lv.4- from hand free."""

    def test_overclock_deletion_allows_play_from_hand(self, debug_runner):
        """When Token/Puppet deleted by Overclock, should draw AND offer play from hand."""
        deck = _build_deck_with(
            [TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4, PUPPET_LV3, PUPPET_LV3]
        )
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=3)

        # Place tamer on field
        runner.place_on_field(1, [TAMER_ID])

        game = runner.game
        p1 = game.player1

        # Create a token to sacrifice via overclock
        game.effect_play_token(p1, 'familiar')
        token_perm = [p for p in p1.battle_area if getattr(p, 'is_token', False)][0]

        # Put a Puppet Lv.4 in hand for free play
        runner.inject_card(1, PUPPET_LV4, zone="hand")

        hand_before = len(p1.hand_cards)
        field_before = len(p1.battle_area)

        # Delete with removal_cause='overclock'
        p1.delete_permanent(token_perm, removal_cause='overclock')

        tamer_perm = [p for p in p1.battle_area if p.is_tamer][0]
        assert tamer_perm.is_suspended, "Tamer should be suspended"

        # Should have drawn 1 (so hand_before + 1 - possibly 1 played = hand_before or hand_before + 1)
        # If the Puppet was played from hand, hand count should be hand_before + 1 (draw) - 1 (play) = hand_before
        # If auto_resolve picks the puppet to play, it will be on field
        # We need to resolve the pending selection for the play
        runner.auto_resolve()

        # After overclock deletion: drew 1, may have played 1 from hand
        # The key test is that the selection was offered
        # Check: field should have tamer + the newly played puppet (token was deleted)
        puppet_on_field = [p for p in p1.battle_area
                          if p.is_digimon and not p.is_tamer
                          and p.top_card
                          and any('Puppet' in t for t in (getattr(p.top_card, 'card_traits', []) or []))]
        assert len(puppet_on_field) >= 1, \
            "Should have played a Puppet from hand via Overclock trigger"

    def test_non_overclock_deletion_does_not_offer_play(self, debug_runner):
        """Regular deletion (not Overclock) should only draw, not offer play."""
        deck = _build_deck_with(
            [TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4, PUPPET_LV3]
        )
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=3)

        runner.place_on_field(1, [TAMER_ID])
        puppet_perm = runner.place_on_field(1, [PUPPET_LV4])

        game = runner.game
        p1 = game.player1

        # Put a Puppet Lv.4 in hand
        runner.inject_card(1, PUPPET_LV4, zone="hand")

        field_count_before = len(p1.battle_area)
        hand_before = len(p1.hand_cards)

        # Delete via regular effect (not Overclock)
        p1.delete_permanent(puppet_perm, removal_cause='effect')

        # Resolve any pending
        runner.auto_resolve()

        # Should have drawn 1, but NOT played from hand
        tamer_perm = [p for p in p1.battle_area if p.is_tamer][0]
        assert tamer_perm.is_suspended, "Tamer should be suspended"

        # Hand should have gained 1 (draw) but not lost any (no play)
        assert len(p1.hand_cards) == hand_before + 1, \
            f"Should have drawn 1 but NOT played from hand. Hand was {hand_before}, now {len(p1.hand_cards)}"


# ---- Test: removal_cause threading ----

class TestRemovalCauseThreading:
    """Verify the engine passes removal_cause through to deletion observers."""

    def test_removal_cause_in_observer_context(self, debug_runner):
        """Deletion observers should receive removal_cause in their context."""
        deck = _build_deck_with([TAMER_ID, TAMER_ID, PUPPET_LV4, PUPPET_LV4])
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=3)

        runner.place_on_field(1, [TAMER_ID])
        puppet_perm = runner.place_on_field(1, [PUPPET_LV4])

        game = runner.game
        p1 = game.player1

        # Monkey-patch the observer's process callback to capture context
        tamer_cs = [p for p in p1.battle_area if p.is_tamer][0].card_sources[0]
        effects = tamer_cs.effect_list(EffectTiming.OnStartTurn)  # returns all effects
        obs_effect = [e for e in effects if getattr(e, '_is_deletion_observer', False)][0]

        captured_causes = []
        original_process = obs_effect.on_process_callback

        def capturing_process(ctx):
            captured_causes.append(ctx.get('removal_cause'))
            return original_process(ctx)

        obs_effect.on_process_callback = capturing_process

        p1.delete_permanent(puppet_perm, removal_cause='overclock')

        assert len(captured_causes) > 0, "Deletion observer should have been triggered"
        assert captured_causes[0] == 'overclock', \
            f"removal_cause should be 'overclock', got {captured_causes[0]}"
