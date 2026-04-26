"""Behavioral tests for BT24-080 Megidramon.

Card text:
This card is also treated as [ChaosGallantmon].
[Trash] [End of Your Turn] If you have 4 or fewer cards in your hand,
    1 of your [Dark Dragon] or [Evil Dragon] Digimon may digivolve into
    this card without paying the cost.
<Blocker>
[On Play] [When Digivolving] [On Deletion] Delete all of your opponent's
    lowest level Digimon.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase

# EX7-056 Orochimon: Lv5 Purple Dark Dragon, 8000 DP (valid digivolve target)
# BT21-077 Regulusmon: Lv5 Purple/Red Evil Dragon, 9000 DP (valid digivolve target)
# ST1-03 Agumon: Lv3, 2000 DP (generic test card)
# ST1-06 Coredramon: Lv4, 6000 DP
MEGIDRAMON_DECK = ["BT24-080"] * 4 + ["EX7-056"] * 23 + ["BT21-077"] * 23
GENERIC_DECK = ["ST1-03"] * 25 + ["ST1-06"] * 25


def _select_target(runner, substring):
    """Find and execute a selection action matching the substring.

    Used during SelectTarget phase to pick a specific permanent instead of
    relying on auto_resolve which picks the first legal action (often Decline).
    """
    action = runner.find_action(substring)
    assert action is not None, (
        f"Could not find action matching '{substring}'. "
        f"Available: {runner.actions()}"
    )
    runner.execute(action)


@pytest.mark.behavioral
class TestBT24080Megidramon:
    """Tests for BT24-080 Megidramon."""

    # ── Clause 0: Also treated as [ChaosGallantmon] ──────────────────

    def test_also_treated_as_chaosgallantmon(self, debug_runner):
        """Megidramon should also be treated as ChaosGallantmon (name aliasing)."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=20)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT24-080"])
        top = perm.top_card
        # Trigger lazy effect initialization (also_treated_as_names is set
        # during get_card_effects which runs on first effect_list call)
        top.effect_list(None)

        assert "ChaosGallantmon" in top.card_names, (
            f"Megidramon should also be treated as ChaosGallantmon, "
            f"but card_names = {top.card_names}"
        )
        assert "Megidramon" in top.card_names, (
            "Megidramon should still have its original name"
        )

    def test_contains_card_name_chaosgallantmon(self, debug_runner):
        """Permanent.contains_card_name should match ChaosGallantmon."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=20)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT24-080"])
        # Trigger lazy effect initialization
        perm.top_card.effect_list(None)

        assert perm.contains_card_name("ChaosGallantmon"), (
            "Permanent should match ChaosGallantmon via contains_card_name"
        )
        assert perm.contains_card_name("Megidramon"), (
            "Permanent should still match Megidramon"
        )

    # ── Clause 1: [Trash] [End of Your Turn] Digivolve from trash ────

    def test_trash_eot_digivolve_onto_dark_dragon(self, debug_runner):
        """End of turn: Megidramon in trash digivolves onto a Dark Dragon Digimon."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=3)
        runner.set_phase("Main")

        game = runner.game
        runner.place_on_field(1, ["EX7-056"], turn_played=-1)  # Lv5 Purple Dark Dragon
        runner.inject_card(1, "BT24-080", "trash")
        runner.clear_zone(1, "hand")

        snap_before = runner.snapshot()
        assert "BT24-080" in snap_before.p1_trash, "Megidramon should be in trash"

        game.execute_effects(EffectTiming.OnEndTurn)

        # The effect creates a selection — pick the Orochimon
        assert game.current_phase == GamePhase.SelectTarget
        _select_target(runner, "Orochimon")
        runner.auto_resolve()

        snap = runner.snapshot()
        megidramon_on_field = [s for s in snap.p1_field if s.card_id == "BT24-080"]
        assert len(megidramon_on_field) == 1, (
            f"Megidramon should be on field after digivolving from trash, "
            f"got {len(megidramon_on_field)}"
        )
        stack = megidramon_on_field[0].stack_ids
        assert "EX7-056" in stack, "Orochimon should be in the digivolution stack"
        assert "BT24-080" in stack, "Megidramon should be in the digivolution stack"
        assert "BT24-080" not in snap.p1_trash, (
            "Megidramon should have been removed from trash"
        )

    def test_trash_eot_digivolve_onto_evil_dragon(self, debug_runner):
        """End of turn: Megidramon in trash digivolves onto an Evil Dragon Digimon."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=3)
        runner.set_phase("Main")

        game = runner.game
        runner.place_on_field(1, ["BT21-077"], turn_played=-1)  # Lv5 Purple/Red Evil Dragon
        runner.inject_card(1, "BT24-080", "trash")
        runner.clear_zone(1, "hand")

        game.execute_effects(EffectTiming.OnEndTurn)

        assert game.current_phase == GamePhase.SelectTarget
        _select_target(runner, "Regulusmon")
        runner.auto_resolve()

        snap = runner.snapshot()
        megidramon_on_field = [s for s in snap.p1_field if s.card_id == "BT24-080"]
        assert len(megidramon_on_field) == 1, (
            "Megidramon should digivolve onto Evil Dragon Digimon"
        )

    def test_trash_eot_not_triggered_with_5_cards_in_hand(self, debug_runner):
        """End of turn: effect should NOT trigger if hand has 5+ cards."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=3)
        runner.set_phase("Main")

        game = runner.game
        runner.place_on_field(1, ["EX7-056"], turn_played=-1)
        runner.inject_card(1, "BT24-080", "trash")
        runner.clear_zone(1, "hand")
        for _ in range(5):
            runner.inject_card(1, "ST1-03", "hand")

        snap_before = runner.snapshot()
        assert len(snap_before.p1_hand) == 5, "Hand should have 5 cards"

        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "BT24-080" in snap.p1_trash, (
            "Megidramon should remain in trash when hand > 4"
        )

    def test_trash_eot_triggered_with_exactly_4_cards(self, debug_runner):
        """End of turn: effect should trigger with exactly 4 cards in hand."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=3)
        runner.set_phase("Main")

        game = runner.game
        runner.place_on_field(1, ["EX7-056"], turn_played=-1)
        runner.inject_card(1, "BT24-080", "trash")
        runner.clear_zone(1, "hand")
        for _ in range(4):
            runner.inject_card(1, "ST1-03", "hand")

        snap_before = runner.snapshot()
        assert len(snap_before.p1_hand) == 4, "Hand should have exactly 4 cards"

        game.execute_effects(EffectTiming.OnEndTurn)

        assert game.current_phase == GamePhase.SelectTarget
        _select_target(runner, "Orochimon")
        runner.auto_resolve()

        snap = runner.snapshot()
        megidramon_on_field = [s for s in snap.p1_field if s.card_id == "BT24-080"]
        assert len(megidramon_on_field) == 1, (
            "Megidramon should trigger with exactly 4 cards in hand"
        )

    def test_trash_eot_not_triggered_on_opponent_turn(self, debug_runner):
        """End of turn: effect should NOT trigger on opponent's turn."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=3)
        runner.set_phase("Main")

        game = runner.game
        runner.place_on_field(1, ["EX7-056"], turn_played=-1)
        runner.inject_card(1, "BT24-080", "trash")
        runner.clear_zone(1, "hand")

        # Switch to P2's turn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "BT24-080" in snap.p1_trash, (
            "Megidramon should not trigger on opponent's turn"
        )

    def test_trash_eot_no_valid_target_no_trigger(self, debug_runner):
        """End of turn: effect should not fire if no Dark/Evil Dragon on field."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=3)
        runner.set_phase("Main")

        game = runner.game
        # Place a NON Dark/Evil Dragon Digimon (Agumon — Reptile trait)
        runner.place_on_field(1, ["ST1-03"], turn_played=-1)
        runner.inject_card(1, "BT24-080", "trash")
        runner.clear_zone(1, "hand")

        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "BT24-080" in snap.p1_trash, (
            "Megidramon should remain in trash when no valid target exists"
        )

    def test_trash_eot_draws_card_on_digivolve(self, debug_runner):
        """Digivolve from trash should draw 1 card (digivolve bonus)."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=3)
        runner.set_phase("Main")

        game = runner.game
        runner.place_on_field(1, ["EX7-056"], turn_played=-1)
        runner.inject_card(1, "BT24-080", "trash")
        runner.clear_zone(1, "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        game.execute_effects(EffectTiming.OnEndTurn)

        assert game.current_phase == GamePhase.SelectTarget
        _select_target(runner, "Orochimon")
        runner.auto_resolve()

        snap = runner.snapshot()
        assert len(snap.p1_hand) == hand_before + 1, (
            f"Should draw 1 card from digivolve, hand went from "
            f"{hand_before} to {len(snap.p1_hand)}"
        )

    def test_trash_eot_is_optional(self, debug_runner):
        """The trash EOT digivolve should be optional (may digivolve)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT24-080")
        effects = cs.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot) >= 1, "Should have at least 1 OnEndTurn effect"
        assert eot[0].is_optional, "Trash EOT digivolve should be optional"

    # ── Clause 2: Blocker ─────────────────────────────────────────────

    def test_blocker_keyword(self, debug_runner):
        """Megidramon should have <Blocker>."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=20)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT24-080"])
        assert perm.has_keyword("_is_blocker"), (
            "Megidramon should have the Blocker keyword"
        )

    # ── Clause 3: [On Play] Delete all opponent's lowest level Digimon ─

    def test_on_play_deletes_lowest_level_digimon(self, debug_runner):
        """On Play: should delete all of opponent's lowest level Digimon."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=20)
        runner.set_phase("Main")

        runner.place_on_field(2, ["ST1-03"], turn_played=-1)  # Lv3 Agumon #1
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)  # Lv3 Agumon #2
        runner.place_on_field(2, ["ST1-06"], turn_played=-1)  # Lv4 Coredramon

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT24-080", "hand")

        snap_before = runner.snapshot()
        assert len(snap_before.p2_field) == 3, "Opponent should have 3 Digimon"

        action = runner.find_action("Play Megidramon")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_field_ids = [s.card_id for s in snap.p2_field]
        agumon_count = opp_field_ids.count("ST1-03")
        assert agumon_count == 0, (
            f"All Lv3 Agumon should be deleted, but {agumon_count} remain"
        )
        assert "ST1-06" in opp_field_ids, (
            "Lv4 Coredramon should survive (not the lowest level)"
        )

    def test_on_play_deletes_all_at_lowest_level(self, debug_runner):
        """On Play: should delete ALL Digimon at the lowest level, not just 1."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=20)
        runner.set_phase("Main")

        runner.place_on_field(2, ["ST1-03"], turn_played=-1)
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT24-080", "hand")

        action = runner.find_action("Play Megidramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"All Lv3 Digimon should be deleted (all are the lowest), "
            f"but {len(opp_digimon)} remain"
        )

    # ── Clause 4: [When Digivolving] Delete all lowest level ──────────

    def test_when_digivolving_deletes_lowest_level(self, debug_runner):
        """When Digivolving: should delete all opponent's lowest level Digimon."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=3)
        runner.set_phase("Main")

        game = runner.game
        # P1: Lv5 Evil Dragon on field, Megidramon in trash
        runner.place_on_field(1, ["BT21-077"], turn_played=-1)  # Lv5 Evil Dragon
        runner.inject_card(1, "BT24-080", "trash")
        runner.clear_zone(1, "hand")

        # P2: Lv3 and Lv4 Digimon on field
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)  # Lv3
        runner.place_on_field(2, ["ST1-06"], turn_played=-1)  # Lv4

        # Trigger trash EOT digivolve
        game.execute_effects(EffectTiming.OnEndTurn)

        # Select Regulusmon to digivolve into Megidramon
        assert game.current_phase == GamePhase.SelectTarget
        _select_target(runner, "Regulusmon")
        runner.auto_resolve()

        snap = runner.snapshot()
        # Lv3 Agumon should be deleted by WhenDigivolving effect
        agumon_count = sum(1 for s in snap.p2_field if s.card_id == "ST1-03")
        assert agumon_count == 0, (
            f"Lv3 Agumon should be deleted by WhenDigivolving, {agumon_count} remain"
        )
        assert any(s.card_id == "ST1-06" for s in snap.p2_field), (
            "Lv4 Coredramon should survive WhenDigivolving deletion"
        )

    # ── Clause 5: [On Deletion] Delete all lowest level ───────────────

    def test_on_deletion_deletes_lowest_level(self, debug_runner):
        """On Deletion: should delete all opponent's lowest level Digimon."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=20)
        runner.set_phase("Main")

        game = runner.game
        perm = runner.place_on_field(1, ["BT24-080"], turn_played=-1)
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)
        runner.place_on_field(2, ["ST1-06"], turn_played=-1)

        snap_before = runner.snapshot()
        assert len(snap_before.p2_field) == 3

        game.player1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert not any(s.card_id == "BT24-080" for s in snap.p1_field), (
            "Megidramon should be deleted from field"
        )
        agumon_count = sum(1 for s in snap.p2_field if s.card_id == "ST1-03")
        assert agumon_count == 0, (
            f"All Lv3 Agumon should be deleted by On Deletion, {agumon_count} remain"
        )
        assert any(s.card_id == "ST1-06" for s in snap.p2_field), (
            "Lv4 Coredramon should survive On Deletion"
        )

    def test_on_deletion_no_opponents_does_nothing(self, debug_runner):
        """On Deletion: if opponent has no Digimon, no crash."""
        runner = debug_runner(deck1=MEGIDRAMON_DECK, deck2=GENERIC_DECK, initial_memory=20)
        runner.set_phase("Main")

        game = runner.game
        perm = runner.place_on_field(1, ["BT24-080"], turn_played=-1)

        snap_before = runner.snapshot()
        assert len(snap_before.p2_field) == 0

        game.player1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert not any(s.card_id == "BT24-080" for s in snap.p1_field)

    # ── Effect structure verification ─────────────────────────────────

    def test_effect_count(self, debug_runner):
        """Megidramon should have the expected number of effects."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT24-080")
        effects = cs.effect_list(None)

        # Effect 0: Also treated as
        # Effect 1: Trash EOT digivolve (OnEndTurn)
        # Effect 2: Blocker (None timing)
        # Effect 3: On Play (OnEnterFieldAnyone, is_on_play)
        # Effect 4: When Digivolving (OnEnterFieldAnyone, is_when_digivolving)
        # Effect 5: On Deletion (OnDestroyedAnyone, is_on_deletion)
        assert len(effects) == 6, (
            f"Expected 6 effects, got {len(effects)}: "
            f"{[e.effect_name for e in effects]}"
        )

    def test_no_inherited_effects(self, debug_runner):
        """Megidramon has no inherited effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT24-080")
        effects = cs.effect_list(None)
        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) == 0, (
            f"Megidramon should have no inherited effects, found {len(inherited)}"
        )
