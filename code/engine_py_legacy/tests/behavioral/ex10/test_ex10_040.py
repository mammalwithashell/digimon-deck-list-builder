"""Behavioral tests for EX10-040 DemiDevimon (Lv.3, Purple, Evil, DP 1000, Cost 3).

Card text:
  [Start of Your Main Phase] If your opponent has 10 or fewer cards in
      their trash, trash the top 2 cards of both players' decks. Then,
      if they have 10 or more cards in their trash, gain 1 memory.
  Inherited: [When Attacking] [Once Per Turn] Trash the top card of both
      players' decks.

Key faithfulness points tested:
  - SoYMP: only mills if opponent trash <= 10
  - SoYMP: mills 2 from BOTH players' decks (turn player first)
  - SoYMP: memory gain checks opponent trash >= 10 AFTER the mill
  - SoYMP: mills 2 from each, not 2 total
  - SoYMP: does NOT fire if opponent trash > 10 (no mill, but still checks memory)
  - SoYMP: boundary case at exactly 10 cards in opponent trash (should mill)
  - SoYMP: boundary case at exactly 10 cards after mill (should gain memory)
  - SoYMP: uses player.mill() to fire OnDiscardLibrary timing
  - Inherited: OPT when attacking, mills 1 from each player's deck
  - Inherited: once per turn enforcement
  - Inherited: uses player.mill() for proper timing
  - Condition: only activates on owner's turn
  - Condition: requires card to be on a permanent (in battle area)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# Card IDs used in tests
EX10_040 = "EX10-040"  # DemiDevimon (card under test)
AGUMON_ST1 = "ST1-03"  # Agumon (Lv.3, filler)
GREYMON_ST1 = "ST1-07"  # Greymon (Lv.4, filler for inherited test)


def _get_soymp_effect(card):
    """Get the Start of Your Main Phase effect from the card."""
    effects = card.effect_list(None)
    soymp = [e for e in effects
             if e.timing == EffectTiming.OnStartMainPhase
             and not getattr(e, 'is_inherited_effect', False)]
    assert len(soymp) == 1, f"Expected 1 SoYMP effect, found {len(soymp)}"
    return soymp[0]


def _get_inherited_attack_effect(card):
    """Get the inherited When Attacking effect from the card."""
    effects = card.effect_list(None)
    attack = [e for e in effects
              if getattr(e, 'is_on_attack', False)
              and getattr(e, 'is_inherited_effect', False)]
    assert len(attack) == 1, f"Expected 1 inherited attack effect, found {len(attack)}"
    return attack[0]


def _populate_trash(player, count):
    """Move cards from library to trash to reach a specific trash count."""
    while len(player.trash_cards) < count and player.library_cards:
        card = player.library_cards.pop(0)
        player.trash_cards.append(card)


@pytest.mark.behavioral
class TestEX10040SoYMPMill:
    """[Start of Your Main Phase] Conditional mill: opponent trash <= 10."""

    def test_mills_2_from_both_decks_when_opponent_trash_lte_10(self, debug_runner):
        """Should trash top 2 of both players' decks when opponent has <= 10 trash."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        # Opponent has 0 cards in trash (well under 10)
        assert len(p2.trash_cards) == 0

        p1_deck_before = len(p1.library_cards)
        p2_deck_before = len(p2.library_cards)

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(p1.library_cards) == p1_deck_before - 2, (
            f"P1 deck should lose 2 cards, was {p1_deck_before}, now {len(p1.library_cards)}")
        assert len(p2.library_cards) == p2_deck_before - 2, (
            f"P2 deck should lose 2 cards, was {p2_deck_before}, now {len(p2.library_cards)}")
        # Both players should have 2 new cards in trash
        assert len(p1.trash_cards) == 2, f"P1 trash should have 2, got {len(p1.trash_cards)}"
        assert len(p2.trash_cards) == 2, f"P2 trash should have 2, got {len(p2.trash_cards)}"

    def test_mills_when_opponent_trash_exactly_10(self, debug_runner):
        """Boundary: opponent has exactly 10 trash cards -> should still mill."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        _populate_trash(p2, 10)
        assert len(p2.trash_cards) == 10

        p1_deck_before = len(p1.library_cards)
        p2_deck_before = len(p2.library_cards)

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(p1.library_cards) == p1_deck_before - 2, "P1 deck should lose 2"
        assert len(p2.library_cards) == p2_deck_before - 2, "P2 deck should lose 2"

    def test_no_mill_when_opponent_trash_over_10(self, debug_runner):
        """Should NOT mill when opponent has > 10 cards in trash."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        _populate_trash(p2, 11)
        assert len(p2.trash_cards) == 11

        p1_deck_before = len(p1.library_cards)
        p2_deck_before = len(p2.library_cards)

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(p1.library_cards) == p1_deck_before, "P1 deck should be unchanged"
        assert len(p2.library_cards) == p2_deck_before, "P2 deck should be unchanged"


@pytest.mark.behavioral
class TestEX10040SoYMPMemory:
    """[Start of Your Main Phase] Conditional memory gain: opponent trash >= 10 after mill."""

    def test_gains_memory_when_opponent_trash_reaches_10_after_mill(self, debug_runner):
        """Should gain 1 memory when opponent reaches 10+ trash after mill.
        Opponent starts with 8 trash + 2 milled = 10 -> gain memory."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        _populate_trash(p2, 8)
        assert len(p2.trash_cards) == 8

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # Opponent should now have 10 trash (8 + 2 milled)
        assert len(p2.trash_cards) == 10
        # P1 is the turn player, memory should increase by 1
        assert game.memory == 1, f"Should gain 1 memory, got {game.memory}"

    def test_gains_memory_when_opponent_already_has_10_plus_and_mills(self, debug_runner):
        """Opponent has exactly 10 trash (still mills because <= 10), ends with 12.
        Should gain 1 memory because 12 >= 10."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        _populate_trash(p2, 10)

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(p2.trash_cards) == 12, "Opponent should have 12 trash (10 + 2 milled)"
        assert game.memory == 1, f"Should gain 1 memory, got {game.memory}"

    def test_no_memory_when_opponent_trash_stays_under_10(self, debug_runner):
        """Opponent has 0 trash + 2 milled = 2. Should NOT gain memory."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        assert len(p2.trash_cards) == 0

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(p2.trash_cards) == 2, "Opponent should have 2 trash (0 + 2 milled)"
        assert game.memory == 0, f"Should NOT gain memory, got {game.memory}"

    def test_memory_gain_when_no_mill_but_opponent_has_11(self, debug_runner):
        """Opponent has 11 trash (> 10) so no mill happens, but 11 >= 10 so memory gained."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        _populate_trash(p2, 11)

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # No mill should happen
        assert len(p2.trash_cards) == 11, "Opponent trash should still be 11 (no mill)"
        # But memory should be gained because 11 >= 10
        assert game.memory == 1, f"Should gain 1 memory (11 >= 10), got {game.memory}"

    def test_no_memory_when_opponent_has_exactly_9_after_mill(self, debug_runner):
        """Opponent has 7 trash + 2 milled = 9. Should NOT gain memory (9 < 10)."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        _populate_trash(p2, 7)

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(p2.trash_cards) == 9, "Opponent should have 9 trash (7 + 2)"
        assert game.memory == 0, f"Should NOT gain memory (9 < 10), got {game.memory}"


@pytest.mark.behavioral
class TestEX10040SoYMPMillUsesProperEngine:
    """Mill operations should use player.mill() for proper OnDiscardLibrary timing."""

    def test_mill_fires_on_discard_library_timing(self, debug_runner):
        """Milling should fire OnDiscardLibrary timing for other card interactions."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        # Track whether OnDiscardLibrary fires
        discard_events = []

        original_fire = p1._fire_timing

        def tracking_fire(timing, ctx=None):
            if timing == EffectTiming.OnDiscardLibrary:
                discard_events.append(('p1', ctx))
            return original_fire(timing, ctx)

        original_fire_p2 = p2._fire_timing

        def tracking_fire_p2(timing, ctx=None):
            if timing == EffectTiming.OnDiscardLibrary:
                discard_events.append(('p2', ctx))
            return original_fire_p2(timing, ctx)

        p1._fire_timing = tracking_fire
        p2._fire_timing = tracking_fire_p2

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(discard_events) >= 2, (
            f"OnDiscardLibrary should fire for both players, got {len(discard_events)} events. "
            "Script must use player.mill() instead of manual pop/append."
        )


@pytest.mark.behavioral
class TestEX10040SoYMPCondition:
    """Condition checks for the SoYMP effect."""

    def test_condition_requires_permanent_on_field(self, debug_runner):
        """SoYMP condition should fail if card is not on a permanent."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        card = perm.top_card
        effect = _get_soymp_effect(card)

        # Condition should pass while on field
        assert effect.can_use_condition({}) is True

        # Remove from field
        game.player1.battle_area.remove(perm)
        card._permanent = None

        # Condition should fail
        assert effect.can_use_condition({}) is False

    def test_condition_requires_owners_turn(self, debug_runner):
        """SoYMP condition should fail if it's not the owner's turn."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        card = perm.top_card
        effect = _get_soymp_effect(card)

        # It's P1's turn by default -> should pass
        assert effect.can_use_condition({}) is True

        # Switch turns
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        # Now condition should fail
        assert effect.can_use_condition({}) is False


@pytest.mark.behavioral
class TestEX10040InheritedAttack:
    """Inherited: [When Attacking] [Once Per Turn] Trash top card of both decks."""

    def test_inherited_mills_1_from_both_decks(self, debug_runner):
        """Should trash top 1 card from both players' decks."""
        runner = debug_runner(initial_memory=0)
        # Stack: Agumon (Lv.3) under Greymon (Lv.4) with DemiDevimon as inherited source
        perm = runner.place_on_field(1, [EX10_040, GREYMON_ST1])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        p1_deck_before = len(p1.library_cards)
        p2_deck_before = len(p2.library_cards)
        p1_trash_before = len(p1.trash_cards)
        p2_trash_before = len(p2.trash_cards)

        # Get the inherited effect from the bottom card (EX10_040)
        bottom_card = perm.card_sources[0]  # EX10_040 is at the bottom
        effect = _get_inherited_attack_effect(bottom_card)

        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(p1.library_cards) == p1_deck_before - 1, (
            f"P1 deck should lose 1 card, was {p1_deck_before}, now {len(p1.library_cards)}")
        assert len(p2.library_cards) == p2_deck_before - 1, (
            f"P2 deck should lose 1 card, was {p2_deck_before}, now {len(p2.library_cards)}")
        assert len(p1.trash_cards) == p1_trash_before + 1, "P1 should gain 1 trash card"
        assert len(p2.trash_cards) == p2_trash_before + 1, "P2 should gain 1 trash card"

    def test_inherited_is_once_per_turn(self, debug_runner):
        """The inherited effect should be limited to once per turn."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        card = perm.top_card
        effect = _get_inherited_attack_effect(card)

        assert effect.max_count_per_turn == 1, (
            f"Inherited attack should be Once Per Turn, got max_count_per_turn={effect.max_count_per_turn}"
        )

    def test_inherited_has_correct_hash_string(self, debug_runner):
        """Hash string should be unique for OPT tracking."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        card = perm.top_card
        effect = _get_inherited_attack_effect(card)

        assert effect.hash_string == "TrashTopCard_EX10_040", (
            f"Expected hash_string 'TrashTopCard_EX10_040', got '{effect.hash_string}'"
        )

    def test_inherited_is_marked_as_inherited(self, debug_runner):
        """Effect should be flagged as inherited."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        card = perm.top_card
        effect = _get_inherited_attack_effect(card)

        assert effect.is_inherited_effect is True, "Should be marked as inherited"

    def test_inherited_mills_fires_on_discard_library(self, debug_runner):
        """Inherited mill should use player.mill() for proper timing."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040, GREYMON_ST1])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        discard_events = []

        original_fire = p1._fire_timing

        def tracking_fire(timing, ctx=None):
            if timing == EffectTiming.OnDiscardLibrary:
                discard_events.append(('p1', ctx))
            return original_fire(timing, ctx)

        original_fire_p2 = p2._fire_timing

        def tracking_fire_p2(timing, ctx=None):
            if timing == EffectTiming.OnDiscardLibrary:
                discard_events.append(('p2', ctx))
            return original_fire_p2(timing, ctx)

        p1._fire_timing = tracking_fire
        p2._fire_timing = tracking_fire_p2

        bottom_card = perm.card_sources[0]
        effect = _get_inherited_attack_effect(bottom_card)

        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        assert len(discard_events) >= 2, (
            f"OnDiscardLibrary should fire for both players, got {len(discard_events)} events. "
            "Script must use player.mill() instead of manual pop/append."
        )


@pytest.mark.behavioral
class TestEX10040EdgeCases:
    """Edge cases for empty decks and partial mills."""

    def test_soymp_partial_mill_when_player_has_1_card(self, debug_runner):
        """If a player has only 1 card in deck, should mill only 1 (not crash)."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        # Leave P1 with only 1 library card
        while len(p1.library_cards) > 1:
            p1.library_cards.pop()

        assert len(p1.library_cards) == 1

        p2_deck_before = len(p2.library_cards)

        effect = _get_soymp_effect(perm.top_card)
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # P1 should have milled 1 (all they had), P2 should have milled 2
        assert len(p1.library_cards) == 0, "P1 should have milled their only card"
        assert len(p1.trash_cards) == 1, "P1 should have 1 card in trash"
        assert len(p2.library_cards) == p2_deck_before - 2, "P2 should lose 2"

    def test_inherited_no_crash_on_empty_deck(self, debug_runner):
        """Inherited effect should handle empty decks gracefully."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [EX10_040, GREYMON_ST1])

        game = runner.game
        p1 = game.player1

        # Empty P1's deck
        p1.library_cards.clear()
        p2_deck_before = len(game.player2.library_cards)

        bottom_card = perm.card_sources[0]
        effect = _get_inherited_attack_effect(bottom_card)

        # Should not crash
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # P1 deck still empty (nothing to mill), P2 should lose 1
        assert len(p1.library_cards) == 0
        assert len(game.player2.library_cards) == p2_deck_before - 1
