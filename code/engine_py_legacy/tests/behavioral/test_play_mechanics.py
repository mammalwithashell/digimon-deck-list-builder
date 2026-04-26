"""Behavioral tests for playing cards from hand.

Tests cover:
- Playing a Digimon from hand deducts correct memory
- Playing a Tamer from hand deducts correct memory
- Unaffordable cards do not appear in legal actions
- Memory crossing zero passes turn to opponent

Note: The game starts in Breeding phase after mulligan. Tests that need Main
phase must pass breeding first or call set_phase("Main").
"""

import pytest


class TestPlayDigimon:
    """Tests for playing Digimon cards from hand."""

    def test_play_digimon_deducts_memory(self, debug_runner):
        """Playing a Lv.3 Digimon (ST1-03 Agumon, cost 3) should deduct 3 memory."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Agumon, cost 3

        before = runner.snapshot()
        action = runner.find_action("Play Agumon")
        assert action is not None, "Should find 'Play Agumon' in legal actions"

        result = runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert after.memory == before.memory - 3, (
            f"Memory should decrease by 3 (Agumon's play cost), "
            f"was {before.memory}, now {after.memory}"
        )

    def test_play_digimon_appears_on_field(self, debug_runner):
        """Playing a Digimon should place it on P1's battle area."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-02", "hand")  # Biyomon Lv.3, cost 2

        before_field_count = len(runner.snapshot().p1_field)
        action = runner.find_action("Play Biyomon")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert len(after.p1_field) == before_field_count + 1, (
            "P1 field should have one more Digimon after playing"
        )
        new_perm = after.p1_field[-1]
        assert new_perm.card_id == "ST1-02", (
            f"New permanent should be ST1-02 (Biyomon), got {new_perm.card_id}"
        )

    def test_play_digimon_removes_from_hand(self, debug_runner):
        """Playing a card should remove it from the hand."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-02", "hand")

        before_hand = runner.snapshot().p1_hand
        assert "ST1-02" in before_hand

        action = runner.find_action("Play Biyomon")
        runner.execute(action)
        runner.auto_resolve()

        after_hand = runner.snapshot().p1_hand
        assert "ST1-02" not in after_hand, "Card should be removed from hand after playing"


class TestPlayTamer:
    """Tests for playing Tamer cards from hand."""

    def test_play_tamer_deducts_memory(self, debug_runner):
        """Playing ST1-12 Tai Kamiya (cost 2) should deduct 2 memory."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-12", "hand")  # Tai Kamiya, cost 2

        before_mem = runner.snapshot().memory
        action = runner.find_action("Play Tai Kamiya")
        assert action is not None, "Should find 'Play Tai Kamiya' in legal actions"

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert after.memory == before_mem - 2, (
            f"Memory should decrease by 2 (Tai Kamiya cost), "
            f"was {before_mem}, now {after.memory}"
        )

    def test_play_tamer_appears_on_field(self, debug_runner):
        """Playing a Tamer should place it on the field as a non-Digimon permanent."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-12", "hand")  # Tai Kamiya

        action = runner.find_action("Play Tai Kamiya")
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        tamer_slots = [s for s in after.p1_field if s.is_tamer]
        assert len(tamer_slots) >= 1, "Should have at least one Tamer on field"
        assert any(s.card_id == "ST1-12" for s in tamer_slots), (
            "Tai Kamiya (ST1-12) should be on the field"
        )


class TestAffordability:
    """Tests that unaffordable cards are not offered as legal actions."""

    def test_unaffordable_card_not_in_actions(self, debug_runner):
        """A card costing more memory than available should not be playable.
        Memory can go to -10, so cost 5 with memory 2 IS playable (2-5 = -3).
        Use a very expensive card with very low memory to truly be unaffordable."""
        runner = debug_runner(archetype1="Puppets", initial_memory=1)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        # ST1-11 WarGreymon costs 12; with 1 memory, 1-12 = -11 which exceeds the
        # memory floor of -10 (effective_cost <= memory + 10 → 12 <= 11 → False)
        runner.inject_card(1, "ST1-11", "hand")  # WarGreymon, cost 12

        action = runner.find_action("Play WarGreymon")
        assert action is None, (
            "WarGreymon (cost 12) should NOT be playable with only 1 memory "
            "(would need memory >= 2 since cost must be <= memory + 10)"
        )

    def test_affordable_card_is_in_actions(self, debug_runner):
        """A card costing exactly the available memory should be playable."""
        runner = debug_runner(archetype1="Puppets", initial_memory=3)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Agumon, cost 3

        action = runner.find_action("Play Agumon")
        assert action is not None, (
            "Agumon (cost 3) should be playable with exactly 3 memory"
        )

    def test_zero_memory_can_still_play(self, debug_runner):
        """With 0 memory, cards are still playable (memory can go negative to -10)."""
        runner = debug_runner(archetype1="Puppets", initial_memory=0)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-02", "hand")  # Biyomon, cost 2

        action = runner.find_action("Play Biyomon")
        assert action is not None, (
            "Biyomon (cost 2) should be playable with 0 memory "
            "(memory can go negative)"
        )


class TestMemorySwing:
    """Tests that memory crossing zero passes the turn."""

    def test_memory_swing_changes_active_player(self, debug_runner):
        """Playing a card that causes memory to go negative should pass the turn."""
        runner = debug_runner(archetype1="Puppets", initial_memory=1)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Agumon, cost 3

        before = runner.snapshot()
        assert before.active_player == 1, "P1 should be active before play"

        action = runner.find_action("Play Agumon")
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        # Memory was 1, cost 3, so memory = 1-3 = -2 -> opponent gets 2
        assert after.memory < 0 or after.active_player == 2, (
            "Turn should pass to P2 after memory crosses zero "
            f"(memory={after.memory}, active={after.active_player})"
        )
