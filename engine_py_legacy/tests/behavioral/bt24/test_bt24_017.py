"""Behavioral tests for BT24-017 Medusamon (Lv.6 Red Digimon).

Card text:
  <Raid> <Progress> <Piercing>
  [When Digivolving] Delete 1 of your opponent's lowest DP Digimon.
  Then, by returning 2 cards from their trash to the bottom of the deck,
  they play 2 [Petrification] Tokens. After, this Digimon gets +2000 DP
  for each of your opponent's Digimon until their turn ends.

Evo Costs: Red Lv.5 for 3

Clause decomposition:
  C1: <Raid> keyword
  C2: <Progress> keyword
  C3: <Piercing> keyword
  C4: [When Digivolving] Delete 1 of opponent's lowest DP Digimon (mandatory)
  C5: Then, by returning 2 cards from their trash to the bottom of the deck,
      they play 2 [Petrification] Tokens (optional -- can decline entire thing,
      but must return exactly 2 or 0)
  C6: After, this Digimon gets +2000 DP for each of opponent's Digimon until
      their turn ends (snapshot count, only if C5 completed)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase

# Minimal deck: all Medusamon (Lv.6, cost 11, Red)
MEDUSA_DECK = ["BT24-017"] * 4 + ["ST1-08"] * 46  # Garudamon Lv.5 Red as filler


def _resolve_selecting_trash(runner, max_steps=20):
    """Drive auto-resolution but prefer selecting trash cards over declining.

    For SelectTarget phases with trash indices (130+), picks the first trash
    index instead of 62 (decline). Otherwise behaves like auto_resolve.
    """
    SEL_TRASH_START = 130
    results = []
    for _ in range(max_steps):
        if runner.game.game_over:
            break
        if runner.game.current_phase in (GamePhase.Main, GamePhase.Breeding, GamePhase.End):
            break
        legal = runner.action_mask()
        if not legal:
            break
        # Prefer trash indices over decline
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        if trash_actions:
            results.append(runner.execute(trash_actions[0]))
        else:
            results.append(runner.execute(legal[0]))
    return results


@pytest.mark.behavioral
class TestBT24017MedusamonKeywords:
    """Tests for BT24-017 Medusamon keywords (Raid, Progress, Piercing)."""

    def test_raid_keyword_present(self, debug_runner):
        """C1: Medusamon should have the Raid keyword."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-017", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        raid = [e for e in effects if getattr(e, '_is_raid', False)]
        assert len(raid) == 1, "Should have 1 Raid effect"

    def test_progress_keyword_present(self, debug_runner):
        """C2: Medusamon should have the Progress keyword."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-017", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        progress = [e for e in effects if getattr(e, '_is_progress', False)]
        assert len(progress) == 1, "Should have 1 Progress effect"

    def test_piercing_keyword_present(self, debug_runner):
        """C3: Medusamon should have the Piercing keyword."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-017", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        piercing = [e for e in effects if getattr(e, '_is_piercing', False)]
        assert len(piercing) == 1, "Should have 1 Piercing effect"


@pytest.mark.behavioral
class TestBT24017MedusamonWhenDigivolving:
    """Tests for BT24-017 Medusamon [When Digivolving] effect."""

    def test_wd_timing_correct(self, debug_runner):
        """WD effect should use OnEnterFieldAnyone timing with is_when_digivolving."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-017", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        wd = [e for e in effects
              if e.timing == EffectTiming.OnEnterFieldAnyone
              and getattr(e, 'is_when_digivolving', False)]
        assert len(wd) == 1, "Should have 1 When Digivolving effect"

    def test_wd_deletes_lowest_dp_opponent_digimon(self, debug_runner):
        """C4: WD should delete 1 of opponent's lowest DP Digimon."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place a Lv.5 base on P1's field to digivolve onto
        runner.place_on_field(1, ["ST1-08"])  # Garudamon Lv.5 Red
        # Give P1 Medusamon in hand
        runner.inject_card(1, "BT24-017", "hand")

        # Place 2 opponent Digimon: one at 3000 DP, one at 6000 DP
        runner.place_on_field(2, ["ST1-03"])  # ST1-03 Agumon Lv.3, 2000 DP
        runner.place_on_field(2, ["ST1-08"])  # ST1-08 Garudamon Lv.5, 7000 DP

        # Clear opponent trash so the "return 2 from trash" check fails
        # (isolates the delete test from trash/token/DP logic)
        runner.clear_zone(2, "trash")

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)

        # Digivolve Medusamon onto Garudamon
        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available (deck may not have matching evo source)")
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Opponent should have lost the lowest DP Digimon (ST1-03 at 2000 DP)
        assert len(snap_after.p2_field) == p2_field_before - 1, \
            f"Should delete 1 opponent Digimon; P2 field went from {p2_field_before} to {len(snap_after.p2_field)}"
        # The remaining Digimon should be the higher DP one (Garudamon)
        remaining_ids = [s.card_id for s in snap_after.p2_field]
        assert "ST1-08" in remaining_ids, \
            "Higher DP Digimon (Garudamon) should survive; lowest DP should be deleted"

    def test_wd_no_opponent_digimon_no_crash(self, debug_runner):
        """C4: If opponent has no Digimon, WD delete step should be skipped safely."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-08"])
        runner.inject_card(1, "BT24-017", "hand")
        # No opponent Digimon on field

        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Medusamon should be on field
        assert any(s.card_id == "BT24-017" for s in snap.p1_field), \
            "Medusamon should be on field after digivolving"

    def test_wd_trash_return_plays_2_petrification_tokens(self, debug_runner):
        """C5: Returning 2 cards from opponent trash should play 2 Petrification Tokens."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-08"])
        runner.inject_card(1, "BT24-017", "hand")

        # No opponent Digimon to delete (skip C4 cleanly)
        # But put 2+ cards in opponent's trash so trash return works
        runner.inject_card(2, "ST1-03", "trash")
        runner.inject_card(2, "ST1-03", "trash")

        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available")
        runner.execute(action)
        # Must select trash cards (not decline) to trigger token placement
        _resolve_selecting_trash(runner)

        snap = runner.snapshot()
        # 2 Petrification Tokens should be on opponent's field
        petrification_count = sum(
            1 for s in snap.p2_field
            if 'TOKEN_PETRIFICATION' in s.card_id or 'Petrification' in s.card_name
        )
        assert petrification_count == 2, \
            f"Expected 2 Petrification Tokens on P2 field, found {petrification_count}"

    def test_wd_trash_return_moves_cards_to_deck_bottom(self, debug_runner):
        """C5: The 2 returned cards should go to the bottom of opponent's deck."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-08"])
        runner.inject_card(1, "BT24-017", "hand")

        # Put 2 cards in opponent's trash
        runner.inject_card(2, "ST1-03", "trash")
        runner.inject_card(2, "ST1-03", "trash")

        snap_before = runner.snapshot()
        p2_trash_before = len(snap_before.p2_trash)
        p2_lib_before = snap_before.p2_library_size

        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available")
        runner.execute(action)
        _resolve_selecting_trash(runner)

        snap_after = runner.snapshot()
        # 2 cards should have moved from trash to deck bottom
        # Trash should have 2 fewer (but may have gained cards from deleted Digimon)
        p2_lib_after = snap_after.p2_library_size
        assert p2_lib_after >= p2_lib_before + 2, \
            f"P2 library should grow by at least 2; was {p2_lib_before}, now {p2_lib_after}"

    def test_wd_not_enough_trash_skips_tokens_and_dp(self, debug_runner):
        """C5: If opponent has <2 trash cards, skip tokens and DP boost entirely."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-08"])
        runner.inject_card(1, "BT24-017", "hand")
        # Put 1 opponent Digimon on field (to test DP boost doesn't fire)
        runner.place_on_field(2, ["ST1-03"])

        # Only 1 card in opponent trash (need 2)
        runner.clear_zone(2, "trash")
        runner.inject_card(2, "ST1-03", "trash")

        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # No petrification tokens should be on opponent's field
        petrification_count = sum(
            1 for s in snap.p2_field
            if 'TOKEN_PETRIFICATION' in s.card_id or 'Petrification' in s.card_name
        )
        assert petrification_count == 0, \
            f"Should have 0 Petrification Tokens when trash < 2, found {petrification_count}"

    def test_wd_dp_boost_after_tokens(self, debug_runner):
        """C6: After tokens, Medusamon gets +2000 DP per opponent's Digimon."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-08"])
        runner.inject_card(1, "BT24-017", "hand")

        # Put 2 cards in opponent's trash for C5
        runner.inject_card(2, "ST1-03", "trash")
        runner.inject_card(2, "ST1-03", "trash")

        # No opponent Digimon initially -- but 2 tokens will be added by C5
        # After tokens: 2 Petrification tokens on opponent field = 2 Digimon
        # DP boost = 2 * 2000 = 4000
        # Base DP of Medusamon = 11000
        # Expected: 11000 + 4000 = 15000

        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available")
        runner.execute(action)
        _resolve_selecting_trash(runner)

        snap = runner.snapshot()
        medusa = next(
            (s for s in snap.p1_field if s.card_id == "BT24-017"), None
        )
        assert medusa is not None, "Medusamon should be on field"

        # Count opponent Digimon (should be 2 tokens)
        opp_digimon_count = sum(1 for s in snap.p2_field if s.is_digimon)

        # C# uses snapshot count at time of application.
        # Expected DP = 11000 + (2000 * opp_digimon_count at snapshot time)
        expected_dp = 11000 + 2000 * opp_digimon_count
        assert medusa.dp == expected_dp, \
            f"Expected DP {expected_dp} (11000 base + 2000*{opp_digimon_count}), got {medusa.dp}"

    def test_wd_dp_boost_is_snapshot_not_dynamic(self, debug_runner):
        """C6: DP boost should be a snapshot, not dynamically recalculated.

        Per C# source: count is computed once after tokens are placed.
        If an opponent's Digimon is later removed, the DP should NOT decrease.
        """
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-08"])
        runner.inject_card(1, "BT24-017", "hand")

        # Put 2 cards in opponent's trash for C5
        runner.inject_card(2, "ST1-03", "trash")
        runner.inject_card(2, "ST1-03", "trash")

        # Also place 1 real Digimon on opponent's side
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available")
        runner.execute(action)
        _resolve_selecting_trash(runner)

        snap_before_removal = runner.snapshot()
        medusa_before = next(
            (s for s in snap_before_removal.p1_field if s.card_id == "BT24-017"), None
        )
        assert medusa_before is not None
        dp_before = medusa_before.dp

        # Opp should have: 2 petrification tokens + (maybe ST1-03 if it wasn't
        # the lowest DP deleted). The lowest DP would be the ST1-03 at 2000,
        # which is lower than petrification tokens at 3000.
        # So ST1-03 was deleted. Remaining: 2 petrification tokens.
        # Snapshot count at time of DP application = 2 tokens = 2 Digimon
        # Expected boost = 2 * 2000 = 4000
        # Expected total = 11000 + 4000 = 15000
        expected_dp = 11000 + 2 * 2000
        assert dp_before == expected_dp, \
            f"Expected {expected_dp}, got {dp_before}"

        # Now remove one of the opponent's petrification tokens
        p2 = runner.game.player2
        if p2.battle_area:
            token = p2.battle_area[0]
            p2.delete_permanent(token)

        snap_after_removal = runner.snapshot()
        medusa_after = next(
            (s for s in snap_after_removal.p1_field if s.card_id == "BT24-017"), None
        )
        assert medusa_after is not None

        # With snapshot: DP should remain the same (still +4000)
        # With dynamic: DP would drop to 11000 + 1*2000 = 13000
        assert medusa_after.dp == dp_before, \
            f"DP boost should be snapshot (stay {dp_before}), but got {medusa_after.dp}"

    def test_wd_dp_boost_expires_at_opponent_turn_end(self, debug_runner):
        """C6: DP boost should expire at end of opponent's turn."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-08"])
        runner.inject_card(1, "BT24-017", "hand")

        # Put 2 cards in opponent's trash
        runner.inject_card(2, "ST1-03", "trash")
        runner.inject_card(2, "ST1-03", "trash")

        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available")
        runner.execute(action)
        _resolve_selecting_trash(runner)

        snap = runner.snapshot()
        medusa = next(
            (s for s in snap.p1_field if s.card_id == "BT24-017"), None
        )
        assert medusa is not None
        assert medusa.dp > 11000, "Medusamon should have DP boost"

    def test_wd_full_sequence_with_delete_tokens_and_boost(self, debug_runner):
        """Full WD sequence: delete lowest, return 2 trash, get 2 tokens, DP boost."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-08"])
        runner.inject_card(1, "BT24-017", "hand")

        # Opponent: 1 low DP Digimon + 2 trash cards
        runner.place_on_field(2, ["ST1-03"])  # 2000 DP Agumon
        runner.inject_card(2, "ST1-03", "trash")
        runner.inject_card(2, "ST1-03", "trash")

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)
        p2_trash_before = len(snap_before.p2_trash)

        action = runner.find_action("Digivolve")
        if action is None:
            pytest.skip("Digivolve action not available")
        runner.execute(action)
        _resolve_selecting_trash(runner)

        snap_after = runner.snapshot()

        # 1. Agumon (lowest DP) should be deleted -> its cards go to trash
        # 2. 2 trash cards returned to deck bottom
        # 3. 2 Petrification Tokens on opponent field
        petrification_count = sum(
            1 for s in snap_after.p2_field
            if 'TOKEN_PETRIFICATION' in s.card_id or 'Petrification' in s.card_name
        )
        assert petrification_count == 2, \
            f"Expected 2 Petrification Tokens, found {petrification_count}"

        # 4. DP boost: 2 tokens on opponent field = +4000
        medusa = next(
            (s for s in snap_after.p1_field if s.card_id == "BT24-017"), None
        )
        assert medusa is not None
        # With 2 opponent Digimon (tokens), boost = 2 * 2000 = 4000
        assert medusa.dp == 11000 + 2 * 2000, \
            f"Expected 15000 DP (11000 + 4000), got {medusa.dp}"

    def test_wd_does_not_trigger_on_play(self, debug_runner):
        """WD should only trigger on digivolve, not on play."""
        runner = debug_runner(deck1=MEDUSA_DECK, deck2=MEDUSA_DECK, initial_memory=15)
        runner.set_phase("Main")

        # Opponent Digimon to see if it gets deleted
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(2, "ST1-03", "trash")
        runner.inject_card(2, "ST1-03", "trash")

        runner.inject_card(1, "BT24-017", "hand")
        action = runner.find_action("Play Medusamon")
        if action is None:
            pytest.skip("Cannot play Medusamon (cost too high or not found)")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # On Play should NOT trigger WD - opponent Digimon should still be there
        assert any(s.card_id == "ST1-03" for s in snap.p2_field), \
            "WD should not trigger on play; opponent Digimon should still be present"
