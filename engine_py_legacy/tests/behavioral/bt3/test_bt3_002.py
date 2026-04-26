"""Behavioral tests for BT3-002 DemiVeemon (Lv.2 Digi-Egg, Blue).

Card text (Inherited Effect):
  [When Attacking] [Once Per Turn] If this Digimon has <Jamming>, <Draw 1>
  (Draw 1 card from your deck.)

Clauses:
  1. Timing: [When Attacking] -- triggers when this Digimon attacks
  2. Constraint: [Once Per Turn] -- can only trigger once per turn
  3. Condition: "If this Digimon has <Jamming>" -- gating condition
  4. Effect: <Draw 1> -- draw 1 card from deck
  5. Inherited: This is an inherited effect from a Digi-Egg
"""

import pytest


@pytest.mark.behavioral
class TestBT3002DemiVeemon:
    """Tests for BT3-002 DemiVeemon inherited effect."""

    # ── Clause 5: Inherited effect only fires when BT3-002 is below a top card ──

    def test_effect_is_inherited(self, debug_runner):
        """BT3-002's effect should be marked as inherited."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT3-002")
        effects = cs.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack]
        assert len(on_attack) == 1, "Should have exactly 1 on-attack effect"
        assert on_attack[0].is_inherited_effect, "Effect should be inherited"

    # ── Clause 3+4: With Jamming, attacking draws 1 card ──

    def test_draw_1_when_attacking_with_jamming(self, debug_runner):
        """When attacking with Jamming, should draw 1 card."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        # Build stack: BT3-002 (egg) -> BT16-008 (Lv.4, has non-inherited Jamming)
        # BT16-008 Aquilamon has Jamming as a non-inherited keyword
        perm = runner.place_on_field(1, ["BT3-002", "BT16-008"])

        # Verify this Digimon has Jamming
        assert perm.has_keyword('_is_jamming'), (
            "Stack with BT16-008 on top should have Jamming"
        )

        # Ensure enough cards in deck to draw
        for _ in range(3):
            runner.inject_card(1, "ST2-03", "library_top")

        hand_before = len(runner.game.player1.hand_cards)

        # Attack player
        action = runner.find_action("Attack player")
        assert action is not None, (
            f"Should be able to attack. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before + 1, (
            f"Should have drawn 1 card from BT3-002 inherited effect. "
            f"Hand before: {hand_before}, after: {hand_after}"
        )

    # ── Clause 3: Without Jamming, should NOT draw ──

    def test_no_draw_when_attacking_without_jamming(self, debug_runner):
        """When attacking without Jamming, should NOT draw."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        # Build stack: BT3-002 (egg) -> ST2-03 (Gabumon Lv.3, no Jamming)
        perm = runner.place_on_field(1, ["BT3-002", "ST2-03"])

        # Verify this Digimon does NOT have Jamming
        assert not perm.has_keyword('_is_jamming'), (
            "Stack with ST2-03 on top should NOT have Jamming"
        )

        # Ensure enough cards in deck to draw
        for _ in range(3):
            runner.inject_card(1, "ST2-03", "library_top")

        hand_before = len(runner.game.player1.hand_cards)

        # Attack player
        action = runner.find_action("Attack player")
        assert action is not None, (
            f"Should be able to attack. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, (
            f"Should NOT have drawn (no Jamming). "
            f"Hand before: {hand_before}, after: {hand_after}"
        )

    # ── Clause 2: Once Per Turn constraint ──

    def test_once_per_turn_only_draws_once(self, debug_runner):
        """Effect should only trigger once per turn even if attacking twice."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        # Build stack with Jamming
        perm = runner.place_on_field(1, ["BT3-002", "BT16-008"])

        # Also place a target so we can attack twice
        runner.place_on_field(2, ["ST2-03"])

        # Ensure enough cards in deck
        for _ in range(5):
            runner.inject_card(1, "ST2-03", "library_top")

        hand_before = len(runner.game.player1.hand_cards)

        # First attack
        action = runner.find_action("Attack player")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        hand_after_first = len(runner.game.player1.hand_cards)
        drew_first = hand_after_first - hand_before
        assert drew_first == 1, (
            f"First attack should draw 1. Drew: {drew_first}"
        )

        # Unsuspend attacker for second attack
        perm.is_suspended = False
        runner.set_phase("Main")

        # Second attack same turn
        action2 = runner.find_action("Attack player")
        if action2 is not None:
            runner.execute(action2)
            runner.auto_resolve()

            hand_after_second = len(runner.game.player1.hand_cards)
            drew_second = hand_after_second - hand_after_first
            assert drew_second == 0, (
                f"Second attack should NOT draw (once per turn). "
                f"Drew: {drew_second}"
            )

    # ── Clause 4: Effect draws exactly 1 card ──

    def test_draws_exactly_1_card(self, debug_runner):
        """Effect should draw exactly 1 card, not more or less."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT3-002", "BT16-008"])

        # Ensure deck has cards
        for _ in range(5):
            runner.inject_card(1, "ST2-03", "library_top")

        deck_before = len(runner.game.player1.library_cards)
        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("Attack player")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        deck_after = len(runner.game.player1.library_cards)
        hand_after = len(runner.game.player1.hand_cards)

        assert hand_after - hand_before == 1, "Should draw exactly 1 card to hand"
        assert deck_before - deck_after == 1, "Should remove exactly 1 card from deck"

    # ── Clause 5: Inherited effect only active when BT3-002 is in a stack ──

    def test_inherited_effect_with_granted_jamming(self, debug_runner):
        """Jamming granted via grant_keyword should also satisfy the condition."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")

        # Build stack: BT3-002 (egg) -> ST2-03 (no Jamming)
        perm = runner.place_on_field(1, ["BT3-002", "ST2-03"])

        # Grant Jamming directly
        perm.grant_keyword('_is_jamming')
        assert perm.has_keyword('_is_jamming'), "Should have Jamming after grant"

        for _ in range(3):
            runner.inject_card(1, "ST2-03", "library_top")

        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("Attack player")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before + 1, (
            f"Should draw 1 with granted Jamming. "
            f"Hand before: {hand_before}, after: {hand_after}"
        )
