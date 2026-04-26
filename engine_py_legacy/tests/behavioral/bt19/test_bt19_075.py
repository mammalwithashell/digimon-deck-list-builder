"""Behavioral tests for BT19-075 MoonMillenniummon (Lv.7 Purple, Cost 15, DP 15000).

Card text:
  [On Play] [When Digivolving] Your opponent trashes cards in their hand until
    they have 5 left. For every 2 cards trashed by this effect, delete 1 of
    your opponent's Tamers.
  [All Turns] When this Digimon would leave the battle area, by deleting 1 of
    your Digimon with the [Composite] trait, it doesn't leave.
  [All Turns] [Once Per Turn] When other Digimon or Tamers are deleted, trash
    your opponent's top security card.
  Alt-Digi: From [Millenniummon] for cost 2

Clauses:
  C1: [On Play] triggers hand discard (opponent trashes to 5)
  C2: [When Digivolving] triggers same hand discard
  C3: No discard if opponent has 5 or fewer cards
  C4: Tamer deletion scales: floor(trashed / 2) tamers deleted
  C5: Tamer deletion is capped at available tamers on opponent's field
  C6: Opponent selects which cards to trash (not automatic)
  C7: Active player selects which tamers to delete
  C8: [All Turns] WhenRemoveField -- optional sacrifice of [Composite] Digimon
  C9: Sacrifice must be another [Composite] Digimon (not self)
  C10: On successful sacrifice, this Digimon is prevented from leaving
  C11: If no [Composite] Digimon available, effect cannot activate
  C12: [All Turns] [Once Per Turn] On other Digimon/Tamer deletion, trash 1 opp security
  C13: Does NOT trigger on self being deleted
  C14: Once per turn -- second deletion in same turn should not trigger
  C15: Alt-digi from [Millenniummon] for cost 2
"""

import pytest
from digimon_gym.engine.game.constants import SEL_MY_FIELD_START


# BT2-077 Kimeramon: Lv.5 Purple Digimon, Composite trait
COMPOSITE_DIGIMON = "BT2-077"
# BT1-085 Tai Kamiya: simple Tamer
TAMER_CARD = "BT1-085"
# BT1-010 Agumon: Lv.3 filler Digimon
FILLER = "BT1-010"
# BT1-025: Lv.5 WarGreymon (for opponent Digimon target)
OPP_DIGIMON = "BT1-025"


def _make_runner(debug_runner, **kwargs):
    """Create a DebugRunner with BT19-075 and supporting cards."""
    filler = [FILLER] * 40
    deck1 = ["BT19-075"] * 4 + [COMPOSITE_DIGIMON] * 4 + filler
    deck2 = [TAMER_CARD] * 4 + [OPP_DIGIMON] * 4 + filler
    return debug_runner(
        deck1=deck1,
        deck2=deck2,
        skip_shuffle=True,
        auto_mulligan=True,
        initial_memory=kwargs.get("initial_memory", 10),
    )


# ── Alt-Digi ────────────────────────────────────────────────────────────

@pytest.mark.behavioral
class TestBT19075AltDigi:
    """C15: Alternate digivolution from [Millenniummon] for cost 2."""

    def test_alt_digi_attributes(self, debug_runner):
        """Should have alt-digi cost 2 from Millenniummon."""
        runner = _make_runner(debug_runner)
        card = runner.inject_card(1, "BT19-075", "hand")
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost') and e._alt_digi_cost is not None]
        assert len(alt_digi) == 1, "Should have exactly 1 alt-digi effect"
        assert alt_digi[0]._alt_digi_cost == 2
        assert alt_digi[0]._alt_digi_name == "Millenniummon"


# ── On Play / When Digivolving: Hand Discard ────────────────────────────

@pytest.mark.behavioral
class TestBT19075HandDiscard:
    """C1-C3: Opponent trashes hand to 5 on play/digivolve."""

    def _trigger_on_play(self, runner, mock_hand_select=None, mock_perm_select=None):
        """Place BT19-075 and manually trigger its On Play effect."""
        perm = runner.place_on_field(1, ["BT19-075"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]
        assert len(on_play) >= 1, "Should have On Play effect"

        if mock_hand_select:
            runner.game.effect_select_hand_card = mock_hand_select
        if mock_perm_select:
            runner.game.effect_select_opponent_permanent = mock_perm_select

        ctx = {'player': runner.game.player1, 'game': runner.game}
        on_play[0].on_process_callback(ctx)
        return perm

    def test_on_play_trashes_hand_to_5(self, debug_runner):
        """C1: Opponent with 8 cards should trash 3 to reach 5."""
        runner = _make_runner(debug_runner)

        # Give opponent 8 hand cards
        runner.clear_zone(2, "hand")
        for _ in range(8):
            runner.inject_card(2, FILLER, "hand")
        assert len(runner.game.player2.hand_cards) == 8

        # Auto-select first valid hand card each time
        def auto_hand_select(player, filter_fn, callback, is_optional=False, **kwargs):
            for c in list(player.hand_cards):
                if filter_fn(c):
                    callback(c)
                    return

        self._trigger_on_play(runner, mock_hand_select=auto_hand_select)

        assert len(runner.game.player2.hand_cards) == 5, (
            f"Opponent should have 5 cards after discard. Got {len(runner.game.player2.hand_cards)}")

    def test_on_play_no_discard_at_5(self, debug_runner):
        """C3: No discard if opponent already has 5 cards."""
        runner = _make_runner(debug_runner)

        runner.clear_zone(2, "hand")
        for _ in range(5):
            runner.inject_card(2, FILLER, "hand")

        trash_before = len(runner.game.player2.trash_cards)
        self._trigger_on_play(runner)

        assert len(runner.game.player2.hand_cards) == 5
        assert len(runner.game.player2.trash_cards) == trash_before, (
            "No cards should be trashed when opponent has exactly 5")

    def test_on_play_no_discard_below_5(self, debug_runner):
        """C3: No discard if opponent has fewer than 5 cards."""
        runner = _make_runner(debug_runner)

        runner.clear_zone(2, "hand")
        for _ in range(3):
            runner.inject_card(2, FILLER, "hand")

        trash_before = len(runner.game.player2.trash_cards)
        self._trigger_on_play(runner)

        assert len(runner.game.player2.hand_cards) == 3
        assert len(runner.game.player2.trash_cards) == trash_before

    def test_when_digivolving_also_triggers_discard(self, debug_runner):
        """C2: When Digivolving effect should also discard opponent's hand."""
        runner = _make_runner(debug_runner)

        runner.clear_zone(2, "hand")
        for _ in range(7):
            runner.inject_card(2, FILLER, "hand")
        assert len(runner.game.player2.hand_cards) == 7

        perm = runner.place_on_field(1, ["BT19-075"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving]
        assert len(when_digi) >= 1, "Should have When Digivolving effect"

        def auto_hand_select(player, filter_fn, callback, is_optional=False, **kwargs):
            for c in list(player.hand_cards):
                if filter_fn(c):
                    callback(c)
                    return

        runner.game.effect_select_hand_card = auto_hand_select
        ctx = {'player': runner.game.player1, 'game': runner.game}
        when_digi[0].on_process_callback(ctx)

        assert len(runner.game.player2.hand_cards) == 5, (
            f"When Digivolving should also trash to 5. Got {len(runner.game.player2.hand_cards)}")

    def test_trashed_cards_go_to_trash(self, debug_runner):
        """Trashed cards should end up in opponent's trash zone."""
        runner = _make_runner(debug_runner)

        runner.clear_zone(2, "hand")
        runner.clear_zone(2, "trash")
        for _ in range(7):
            runner.inject_card(2, FILLER, "hand")

        def auto_hand_select(player, filter_fn, callback, is_optional=False, **kwargs):
            for c in list(player.hand_cards):
                if filter_fn(c):
                    callback(c)
                    return

        self._trigger_on_play(runner, mock_hand_select=auto_hand_select)

        assert len(runner.game.player2.trash_cards) == 2, (
            f"2 cards should be in trash. Got {len(runner.game.player2.trash_cards)}")


# ── On Play / When Digivolving: Tamer Deletion ──��──────────────────────

@pytest.mark.behavioral
class TestBT19075TamerDeletion:
    """C4-C7: For every 2 trashed, delete 1 opponent Tamer."""

    def _setup_and_trigger(self, runner, opp_hand_count, tamer_count,
                           mock_hand_select=None, mock_perm_select=None):
        """Set up opponent hand/tamers and trigger On Play effect."""
        runner.clear_zone(2, "hand")
        runner.clear_zone(2, "trash")
        for _ in range(opp_hand_count):
            runner.inject_card(2, FILLER, "hand")

        tamers = []
        for _ in range(tamer_count):
            tamers.append(runner.place_on_field(2, [TAMER_CARD]))

        perm = runner.place_on_field(1, ["BT19-075"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]

        if mock_hand_select:
            runner.game.effect_select_hand_card = mock_hand_select
        else:
            def auto_hand_select(player, filter_fn, callback, is_optional=False, **kwargs):
                for c in list(player.hand_cards):
                    if filter_fn(c):
                        callback(c)
                        return
            runner.game.effect_select_hand_card = auto_hand_select

        if mock_perm_select:
            runner.game.effect_select_opponent_permanent = mock_perm_select
        else:
            def auto_perm_select(player, callback, filter_fn=None, is_optional=False, **kwargs):
                enemy = player.enemy
                for p in enemy.battle_area:
                    if filter_fn is None or filter_fn(p):
                        callback(p)
                        return
            runner.game.effect_select_opponent_permanent = auto_perm_select

        ctx = {'player': runner.game.player1, 'game': runner.game}
        on_play[0].on_process_callback(ctx)
        return tamers

    def test_trash_2_deletes_1_tamer(self, debug_runner):
        """C4: Trashing 2 cards (hand=7, to 5) should delete 1 tamer."""
        runner = _make_runner(debug_runner)
        tamers = self._setup_and_trigger(runner, opp_hand_count=7, tamer_count=2)

        # 7 - 5 = 2 trashed, floor(2/2) = 1 tamer deleted
        remaining = [t for t in tamers if t in runner.game.player2.battle_area]
        assert len(remaining) == 1, (
            f"Should delete 1 tamer (2 trashed). {len(remaining)} remaining")

    def test_trash_4_deletes_2_tamers(self, debug_runner):
        """C4: Trashing 4 cards (hand=9, to 5) should delete 2 tamers."""
        runner = _make_runner(debug_runner)
        tamers = self._setup_and_trigger(runner, opp_hand_count=9, tamer_count=3)

        # 9 - 5 = 4 trashed, floor(4/2) = 2 tamers deleted
        remaining = [t for t in tamers if t in runner.game.player2.battle_area]
        assert len(remaining) == 1, (
            f"Should delete 2 tamers (4 trashed). {len(remaining)} remaining")

    def test_trash_3_deletes_1_tamer(self, debug_runner):
        """C4: Trashing 3 cards (hand=8, to 5) should delete 1 tamer (floor(3/2)=1)."""
        runner = _make_runner(debug_runner)
        tamers = self._setup_and_trigger(runner, opp_hand_count=8, tamer_count=3)

        # 8 - 5 = 3 trashed, floor(3/2) = 1 tamer deleted
        remaining = [t for t in tamers if t in runner.game.player2.battle_area]
        assert len(remaining) == 2, (
            f"Should delete 1 tamer (3 trashed). {len(remaining)} remaining")

    def test_trash_1_deletes_0_tamers(self, debug_runner):
        """C4: Trashing 1 card (hand=6, to 5) should delete 0 tamers (floor(1/2)=0)."""
        runner = _make_runner(debug_runner)
        tamers = self._setup_and_trigger(runner, opp_hand_count=6, tamer_count=2)

        # 6 - 5 = 1 trashed, floor(1/2) = 0 tamers deleted
        remaining = [t for t in tamers if t in runner.game.player2.battle_area]
        assert len(remaining) == 2, (
            f"Should delete 0 tamers (1 trashed). {len(remaining)} remaining")

    def test_tamer_deletion_capped_at_available(self, debug_runner):
        """C5: Can't delete more tamers than exist on the field."""
        runner = _make_runner(debug_runner)
        # 11 - 5 = 6 trashed, floor(6/2) = 3 possible, but only 1 tamer
        tamers = self._setup_and_trigger(runner, opp_hand_count=11, tamer_count=1)

        remaining = [t for t in tamers if t in runner.game.player2.battle_area]
        assert len(remaining) == 0, (
            "Should delete the 1 available tamer even though 3 allowed")

    def test_no_tamers_no_deletion(self, debug_runner):
        """C5: If opponent has no tamers, tamer deletion step is skipped gracefully."""
        runner = _make_runner(debug_runner)
        # 7 - 5 = 2 trashed, floor(2/2) = 1, but 0 tamers on field
        self._setup_and_trigger(runner, opp_hand_count=7, tamer_count=0)
        # Should not raise any errors


# ── WhenRemoveField: Composite Sacrifice ────────────────────────────────

@pytest.mark.behavioral
class TestBT19075CompositeSacrifice:
    """C8-C11: Delete [Composite] Digimon to prevent leaving."""

    def test_prevent_leaving_by_sacrificing_composite(self, debug_runner):
        """C8+C10: Deleting a Composite Digimon prevents this Digimon from leaving."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        composite = runner.place_on_field(1, [COMPOSITE_DIGIMON])

        p1 = runner.game.player1

        # Delete moon via opponent effect
        result = p1.delete_permanent(moon, is_opponent_effect=True)
        assert result is False, "Deletion should be prevented (returned False)"

        # Should enter SelectTarget phase for sacrifice
        assert runner.game.current_phase.name == "SelectTarget", (
            f"Expected SelectTarget, got {runner.game.current_phase.name}")

        # Select the Composite Digimon
        actions = runner.actions()
        field_actions = {aid: desc for aid, desc in actions.items()
                         if aid >= SEL_MY_FIELD_START}
        assert len(field_actions) >= 1, "Should have at least 1 field selection action"
        runner.execute(min(field_actions.keys()))

        # Moon should still be on field
        assert moon in p1.battle_area, (
            "MoonMillenniummon should remain after Composite sacrifice")
        # Composite should be gone
        assert composite not in p1.battle_area, (
            "Sacrificed Composite Digimon should be removed")

    def test_cannot_sacrifice_self(self, debug_runner):
        """C9: Cannot sacrifice self even if BT19-075 is Composite (it isn't, but test filter)."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        # No other Composite on field

        p1 = runner.game.player1

        # Delete moon -- no valid sacrifice (self excluded)
        result = p1.delete_permanent(moon, is_opponent_effect=True)
        assert result is True, "Should be deleted when no sacrifice available"
        assert moon not in p1.battle_area

    def test_no_composite_available_leaves(self, debug_runner):
        """C11: If no [Composite] Digimon available, effect cannot activate."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        # Place a non-Composite Digimon
        runner.place_on_field(1, [FILLER])

        p1 = runner.game.player1
        result = p1.delete_permanent(moon, is_opponent_effect=True)
        assert result is True, "Should be deleted with no Composite available"
        assert moon not in p1.battle_area

    def test_composite_sacrifice_is_optional(self, debug_runner):
        """C8: Player can decline sacrifice (is_optional=True), allowing deletion."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        composite = runner.place_on_field(1, [COMPOSITE_DIGIMON])

        p1 = runner.game.player1
        result = p1.delete_permanent(moon, is_opponent_effect=True)
        assert result is False, "Initially prevented"

        # Decline the sacrifice (Pass action = 62)
        runner.execute(62)

        assert moon not in p1.battle_area, (
            "Should be deleted after declining sacrifice")

    def test_triggers_on_own_effect_removal_too(self, debug_runner):
        """Card doesn't restrict to 'by opponent's effects' -- triggers on any leave."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        composite = runner.place_on_field(1, [COMPOSITE_DIGIMON])

        p1 = runner.game.player1

        # Delete by own effect
        result = p1.delete_permanent(moon, is_opponent_effect=False)
        assert result is False, "Should also trigger on own-effect removal"

        # Select Composite sacrifice
        actions = runner.actions()
        field_actions = {aid: desc for aid, desc in actions.items()
                         if aid >= SEL_MY_FIELD_START}
        if field_actions:
            runner.execute(min(field_actions.keys()))

        assert moon in p1.battle_area, (
            "MoonMillenniummon should survive even from own-effect deletion")


# ── All Turns OPT: Security Trash on Other Deletion ────────────────────

@pytest.mark.behavioral
class TestBT19075SecurityTrash:
    """C12-C14: On other deletion, trash 1 opponent security."""

    def test_other_digimon_deletion_trashes_security(self, debug_runner):
        """C12: Deleting another Digimon should trash 1 opponent security."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        opp_digimon = runner.place_on_field(2, [OPP_DIGIMON])

        sec_before = len(runner.game.player2.security_cards)
        assert sec_before > 0, "Opponent should have security cards"

        # Delete the opponent Digimon
        runner.game.player2.delete_permanent(opp_digimon)

        sec_after = len(runner.game.player2.security_cards)
        assert sec_after == sec_before - 1, (
            f"Should trash 1 security. Before: {sec_before}, After: {sec_after}")

    def test_tamer_deletion_trashes_security(self, debug_runner):
        """C12: Deleting a Tamer should also trigger security trash."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        opp_tamer = runner.place_on_field(2, [TAMER_CARD])

        sec_before = len(runner.game.player2.security_cards)

        runner.game.player2.delete_permanent(opp_tamer)

        sec_after = len(runner.game.player2.security_cards)
        assert sec_after == sec_before - 1, (
            f"Tamer deletion should also trash security. Before: {sec_before}, After: {sec_after}")

    def test_self_deletion_does_not_trigger(self, debug_runner):
        """C13: Deleting MoonMillenniummon itself should NOT trigger the effect."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])

        sec_before = len(runner.game.player2.security_cards)

        # Delete moon itself (first remove Composite prevention by having none)
        runner.game.player1.delete_permanent(moon)

        sec_after = len(runner.game.player2.security_cards)
        assert sec_after == sec_before, (
            f"Self-deletion should NOT trash security. Before: {sec_before}, After: {sec_after}")

    def test_once_per_turn(self, debug_runner):
        """C14: Second deletion in same turn should NOT trigger again."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        opp1 = runner.place_on_field(2, [OPP_DIGIMON])
        opp2 = runner.place_on_field(2, [OPP_DIGIMON])

        sec_before = len(runner.game.player2.security_cards)

        # First deletion
        runner.game.player2.delete_permanent(opp1)
        sec_after_1 = len(runner.game.player2.security_cards)
        assert sec_after_1 == sec_before - 1, "First deletion should trash 1 security"

        # Second deletion in same turn
        runner.game.player2.delete_permanent(opp2)
        sec_after_2 = len(runner.game.player2.security_cards)
        assert sec_after_2 == sec_after_1, (
            f"Second deletion should NOT trash (OPT). After 1st: {sec_after_1}, After 2nd: {sec_after_2}")

    def test_no_security_graceful(self, debug_runner):
        """C12: If opponent has 0 security, effect should not crash."""
        runner = _make_runner(debug_runner)

        moon = runner.place_on_field(1, ["BT19-075"])
        opp_digimon = runner.place_on_field(2, [OPP_DIGIMON])

        # Clear opponent's security
        runner.clear_zone(2, "security")
        assert len(runner.game.player2.security_cards) == 0

        # Should not raise
        runner.game.player2.delete_permanent(opp_digimon)
        assert len(runner.game.player2.security_cards) == 0
