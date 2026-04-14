"""Behavioral tests for BT12-002 DemiVeemon.

Card text:
  Lv.2 | Blue | In-Training, Dragon | DigiEgg | DP: None | Cost: 0
  Inherited Effect [When Attacking] [Once Per Turn] If you have a green
  Digimon in play, <Draw 1> (Draw 1 card from your deck.)

Key faithfulness points tested:
  - Effect is inherited-only (fires from digivolution stack, not standalone).
  - Triggers on [When Attacking] (OnUseAttack timing).
  - [Once Per Turn] — does not fire more than once per turn.
  - Condition: owner must have a green Digimon on their battle area.
  - The green Digimon can be the attacking Digimon itself (if it is green).
  - The green Digimon can be a different Digimon on the owner's field.
  - Action: Draw 1 card from deck.
  - Does NOT fire if no green Digimon in play.
  - Does NOT fire if deck is empty (C# checks LibraryCards.Count >= 1).
"""

import pytest

# Card IDs
BT12_002 = "BT12-002"      # Card under test (DemiVeemon, Lv.2 DigiEgg Blue)
GABUMON = "BT1-029"         # Gabumon (Lv.3 Blue, for main stack)
GOBLIMON = "BT1-064"        # Goblimon (Lv.3 Green Digimon)
AGUMON_ST1 = "ST1-03"       # Agumon (Lv.3 Red, filler)

# Filler deck: must have enough cards for game setup
FILLER = [AGUMON_ST1] * 50


@pytest.mark.behavioral
class TestBT12002InheritedDraw:
    """Inherited [When Attacking] [Once Per Turn] — Draw 1 if green Digimon in play."""

    def test_draws_when_green_digimon_in_play(self, debug_runner):
        """When attacking with a Digimon that has BT12-002 inherited, and a green
        Digimon is in play, should draw 1 card."""
        runner = debug_runner(
            deck1=[BT12_002] * 4 + [GABUMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place Gabumon with BT12-002 as inherited source on field
        # Stack: BT12-002 (bottom) -> Gabumon (top)
        runner.place_on_field(1, [BT12_002, GABUMON])

        # Place a green Digimon (Goblimon) on the field
        runner.place_on_field(1, [GOBLIMON])

        # Give opponent security so attack doesn't end the game
        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        hand_before = len(before.p1_hand)
        lib_before = before.p1_library_size

        # Attack with Gabumon
        attack_action = runner.find_action("Attack")
        assert attack_action is not None, (
            f"Gabumon should be able to attack. Actions: {runner.actions()}"
        )
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        # Should have drawn 1 card from the inherited effect
        assert after.p1_library_size < lib_before, (
            f"Library should decrease from draw. Before: {lib_before}, after: {after.p1_library_size}"
        )
        # Hand should have 1 more card (from the draw)
        assert len(after.p1_hand) >= hand_before + 1, (
            f"Hand should grow by at least 1. Before: {hand_before}, after: {len(after.p1_hand)}"
        )

    def test_no_draw_without_green_digimon(self, debug_runner):
        """When attacking but no green Digimon in play, should NOT draw."""
        runner = debug_runner(
            deck1=[BT12_002] * 4 + [GABUMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place Gabumon with BT12-002 inherited — NO green Digimon on field
        runner.place_on_field(1, [BT12_002, GABUMON])

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        hand_before = len(before.p1_hand)

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        # Should NOT have drawn (no green Digimon)
        assert len(after.p1_hand) == hand_before, (
            f"Without green Digimon, hand should not grow. Before: {hand_before}, after: {len(after.p1_hand)}"
        )

    def test_once_per_turn(self, debug_runner):
        """[Once Per Turn] — inherited effect should not fire more than once per turn,
        even if the Digimon attacks again (e.g. via <Rush> or unsuspend)."""
        runner = debug_runner(
            deck1=[BT12_002] * 4 + [GABUMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place Gabumon with BT12-002 inherited
        perm = runner.place_on_field(1, [BT12_002, GABUMON])
        # Place a green Digimon
        runner.place_on_field(1, [GOBLIMON])

        for _ in range(5):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        hand_before = len(before.p1_hand)
        lib_before = before.p1_library_size

        # First attack
        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after_first = runner.snapshot()
        hand_after_first = len(after_first.p1_hand)
        draws_first = hand_after_first - hand_before

        # Unsuspend and attack again
        perm.unsuspend()
        runner.set_phase("Main")

        attack_action2 = runner.find_action("Attack")
        if attack_action2 is not None:
            lib_before_second = after_first.p1_library_size
            runner.execute(attack_action2)
            runner.auto_resolve()

            after_second = runner.snapshot()
            # The second attack should NOT trigger another draw (OPT)
            # Hand may change from security effects but library should not
            # decrease by more than what security checking costs
            # Most reliable check: the effect fired once, so only 1 draw total
            total_draws = lib_before - after_second.p1_library_size
            # total_draws includes security checks (which move deck->security)
            # but security was pre-loaded, so deck decreases only from draws
            # Actually, security checks don't draw from deck. Only the effect does.
            # So deck should have decreased by exactly 1 (from the first attack's draw)
            # Second attack should not draw again.
            assert draws_first >= 1, "First attack should have drawn 1"
            # After second attack, no additional draw from inherited effect
            hand_after_second = len(after_second.p1_hand)
            # The hand may change due to security battle effects, but the library
            # should only have decreased by 1 total from the inherited effect
            assert after_second.p1_library_size >= after_first.p1_library_size, (
                f"Library should not decrease further from OPT effect on second attack. "
                f"After first: {after_first.p1_library_size}, after second: {after_second.p1_library_size}"
            )

    def test_green_digimon_is_the_attacker_itself(self, debug_runner):
        """If the attacking Digimon itself is green (or has green in its colors),
        it counts as 'a green Digimon in play' for the condition."""
        runner = debug_runner(
            deck1=[BT12_002] * 4 + [GOBLIMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place Goblimon (green) with BT12-002 inherited
        # The attacker IS the green Digimon
        runner.place_on_field(1, [BT12_002, GOBLIMON])

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        hand_before = len(before.p1_hand)

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        # Should draw because the attacker itself is green
        assert len(after.p1_hand) >= hand_before + 1, (
            f"Green attacker should satisfy condition. Hand before: {hand_before}, after: {len(after.p1_hand)}"
        )

    def test_no_draw_when_deck_empty(self, debug_runner):
        """If the deck is empty, the effect should NOT activate (C# checks
        LibraryCards.Count >= 1 in CanActivateCondition). This prevents
        unnecessary deck-out losses."""
        runner = debug_runner(
            deck1=[BT12_002] * 4 + [GABUMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place Gabumon with BT12-002 inherited + green Digimon
        runner.place_on_field(1, [BT12_002, GABUMON])
        runner.place_on_field(1, [GOBLIMON])

        # Empty the deck
        runner.clear_zone(1, "library")

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        assert before.p1_library_size == 0, "Deck should be empty"

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        # The effect should NOT have activated at all — deck was empty.
        # The game should NOT be over from a deck-out caused by the effect
        # trying to draw from an empty deck. The C# CanActivateCondition
        # prevents activation when library is empty.
        assert not after.is_game_over, (
            "Game should NOT end from deck-out — the effect should not activate "
            "when the deck is empty. C# checks LibraryCards.Count >= 1."
        )

    def test_is_inherited_effect_only(self, debug_runner):
        """BT12-002 is a DigiEgg — its effect is inherited only. It should
        not fire as a standalone effect from the egg deck or breeding area."""
        runner = debug_runner(
            deck1=[BT12_002] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )

        # Check that the effect has is_inherited_effect = True
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(BT12_002, runner.game.player1)
        effects = cs.effect_list(None)

        for eff in effects:
            assert eff.is_inherited_effect is True, (
                f"All effects on BT12-002 should be inherited. Found non-inherited: {eff.effect_name}"
            )
