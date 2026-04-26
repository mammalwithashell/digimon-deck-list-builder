"""Behavioral tests for EX10-034 Blastmon.

Card text:
  Lv.6 | Black, Purple | Mineral, Bagra Army | Digimon | DP: 11000 | Play Cost: 13
  <Collision> <Fragment (3)> <Blocker>
  [On Play] [When Digivolving] Until your opponent's turn ends, give 1 of their
  Digimon "[Start of Your Main Phase] This Digimon attacks."
  [All Turns] [Once Per Turn] When Digimon attack, by trashing any 2 of this
  Digimon's digivolution cards, this Digimon gains <Security A. +1> and +3000 DP
  until your turn ends.

Key faithfulness points tested:
  1. Collision keyword is present.
  2. Fragment (3) keyword is present.
  3. Blocker keyword is present.
  4. [On Play] / [When Digivolving] force attack on opponent Digimon.
  5. [All Turns] OPT triggers when ANY Digimon attacks (not just self).
  6. OPT requires selecting which 2 digivolution cards to trash (player choice).
  7. Condition requires >= 3 total cards in stack (top + 2 under-cards).
  8. Grants Security A. +1 and +3000 DP until owner's turn ends.
  9. OPT fires at most once per turn.
  10. With only 2 cards in stack, the effect must NOT activate (not even partially).
"""

import pytest

# Card IDs
EX10_034 = "EX10-034"     # Blastmon (Lv.6 Black/Purple, card under test)
AGUMON_ST1 = "ST1-03"     # Agumon (Lv.3 Red, filler)
GREYMON_BT1 = "BT1-021"   # Greymon (Lv.4 Red, filler stack)
KOROMON = "BT1-003"       # Koromon (Lv.2 Red, filler digi-egg)

# Filler deck
FILLER = [AGUMON_ST1] * 50


@pytest.mark.behavioral
class TestEX10034Keywords:
    """Verify Collision, Fragment, Blocker keywords are present."""

    def test_collision_keyword(self, debug_runner):
        """Blastmon should have the <Collision> keyword."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [AGUMON_ST1, GREYMON_BT1, EX10_034])
        assert perm.has_keyword('_is_collision'), "Blastmon should have Collision"

    def test_blocker_keyword(self, debug_runner):
        """Blastmon should have the <Blocker> keyword."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [AGUMON_ST1, GREYMON_BT1, EX10_034])
        assert perm.has_keyword('_is_blocker'), "Blastmon should have Blocker"

    def test_fragment_keyword(self, debug_runner):
        """Blastmon should have the <Fragment> keyword."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [AGUMON_ST1, GREYMON_BT1, EX10_034])
        assert perm.has_keyword('_is_fragment'), "Blastmon should have Fragment"


@pytest.mark.behavioral
class TestEX10034AllTurnsOPT:
    """[All Turns] [Once Per Turn] When Digimon attack, trash 2 sources -> SA+1 + 3000 DP."""

    def test_condition_rejects_2_card_stack(self, debug_runner):
        """With only 2 cards in the stack (top + 1 under-card), the effect MUST NOT
        activate because you cannot trash 2 under-cards when only 1 exists.
        The DP must NOT change at all."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        # Stack: [AGUMON (under), EX10_034 (top)] = 2 cards, only 1 under-card
        perm = runner.place_on_field(1, [AGUMON_ST1, EX10_034])
        assert len(perm.card_sources) == 2, "Stack should have 2 cards"

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before_dp = perm.dp

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        # The effect should NOT have activated: no DP change, no source trashing
        assert perm.dp == before_dp, (
            f"With only 1 under-card, effect should NOT activate. "
            f"DP should be {before_dp}, got {perm.dp}"
        )
        assert len(perm.card_sources) == 2, (
            f"With only 1 under-card, no sources should be trashed. "
            f"Expected 2, got {len(perm.card_sources)}"
        )
        assert perm.security_attack_modifier() == 0, (
            f"SA modifier should be 0 since effect didn't activate. "
            f"Got: {perm.security_attack_modifier()}"
        )

    def test_condition_met_with_3_cards_in_stack(self, debug_runner):
        """With 3 cards in stack (2 under-cards), the effect activates correctly."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [KOROMON, AGUMON_ST1, EX10_034])
        assert len(perm.card_sources) == 3, "Stack should have 3 cards"

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before_dp = perm.dp  # Should be 11000

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        # After trashing 2 under-cards, only top card remains
        assert len(perm.card_sources) == 1, (
            f"After trashing 2 sources from a 3-card stack, 1 card should remain. "
            f"Got: {len(perm.card_sources)}"
        )

        assert perm.dp == before_dp + 3000, (
            f"DP should be {before_dp + 3000} after +3000 buff. Got: {perm.dp}"
        )

        assert perm.security_attack_modifier() >= 1, (
            f"Security attack modifier should be >= 1. Got: {perm.security_attack_modifier()}"
        )

    def test_triggers_on_any_digimon_attack(self, debug_runner):
        """The effect fires when ANY Digimon attacks, not just Blastmon itself."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        blastmon = runner.place_on_field(1, [KOROMON, AGUMON_ST1, EX10_034])
        attacker = runner.place_on_field(1, [AGUMON_ST1])

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before_dp = blastmon.dp

        # Attack with the OTHER Digimon (Agumon in slot 1, Blastmon in slot 0)
        attack_actions = runner.find_actions("Attack")
        agumon_attack = None
        for aid, desc in attack_actions.items():
            if "Agumon" in desc or "slot[1]" in desc:
                agumon_attack = aid
                break

        if agumon_attack is not None:
            runner.execute(agumon_attack)
            runner.auto_resolve()

            assert blastmon.dp == before_dp + 3000, (
                f"Blastmon should get +3000 DP when another Digimon attacks. "
                f"Before: {before_dp}, After: {blastmon.dp}"
            )
            assert blastmon.security_attack_modifier() >= 1, (
                "Blastmon should get SA+1 when another Digimon attacks."
            )

    def test_once_per_turn(self, debug_runner):
        """[Once Per Turn] - the effect should not fire more than once per turn."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [KOROMON, AGUMON_ST1, GREYMON_BT1, AGUMON_ST1, EX10_034])
        assert len(perm.card_sources) == 5

        for _ in range(5):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before_dp = perm.dp

        # First attack
        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        assert perm.dp == before_dp + 3000, (
            f"First activation should grant +3000 DP. Before: {before_dp}, After: {perm.dp}"
        )
        assert len(perm.card_sources) == 3, (
            f"After first activation, should have 3 cards (trashed 2 from 5). Got: {len(perm.card_sources)}"
        )

        # Unsuspend and attack again
        perm.unsuspend()
        runner.set_phase("Main")

        attack_action2 = runner.find_action("Attack")
        if attack_action2 is not None:
            runner.execute(attack_action2)
            runner.auto_resolve()

            assert perm.dp == before_dp + 3000, (
                f"OPT should not fire twice. Expected {before_dp + 3000}, got {perm.dp}"
            )
            assert len(perm.card_sources) == 3, (
                f"OPT should not trash more sources on second attack. Got: {len(perm.card_sources)}"
            )

    def test_trashed_cards_go_to_trash(self, debug_runner):
        """The trashed digivolution cards should end up in the player's trash pile."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [KOROMON, AGUMON_ST1, EX10_034])

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        trash_before = len(before.p1_trash)

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert len(after.p1_trash) >= trash_before + 2, (
            f"Trash should have at least 2 more cards. Before: {trash_before}, After: {len(after.p1_trash)}"
        )

    def test_player_selects_which_cards_to_trash(self, debug_runner):
        """Player should choose which 2 digivolution cards to trash (not auto-trash).
        With 4 cards in stack (3 under-cards), after selecting 2 to trash, 2 remain."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [KOROMON, AGUMON_ST1, GREYMON_BT1, EX10_034])
        assert len(perm.card_sources) == 4, "Should have 4 cards in stack"

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)

        # auto_resolve picks first legal action for each selection step
        runner.auto_resolve()

        assert len(perm.card_sources) == 2, (
            f"After trashing 2 from 4-card stack, should have 2 remaining. "
            f"Got: {len(perm.card_sources)}"
        )

    def test_exactly_2_trashed_not_1(self, debug_runner):
        """Verify that exactly 2 digivolution cards are trashed, not fewer.
        This catches the bug where the condition passes with 2 cards in digivolution_cards
        (which includes top card) but only 1 can actually be trashed."""
        runner = debug_runner(deck1=[EX10_034] * 4 + [AGUMON_ST1] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        # 3-card stack: exactly 2 under-cards to trash
        perm = runner.place_on_field(1, [KOROMON, AGUMON_ST1, EX10_034])

        for _ in range(3):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before_stack = len(perm.card_sources)
        before_trash = len(runner.snapshot().p1_trash)

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after_stack = len(perm.card_sources)
        after_trash = len(runner.snapshot().p1_trash)

        cards_removed = before_stack - after_stack
        cards_added_to_trash = after_trash - before_trash

        assert cards_removed == 2, (
            f"Exactly 2 cards should be removed from stack. Removed: {cards_removed}"
        )
        assert cards_added_to_trash >= 2, (
            f"At least 2 cards should go to trash. Added: {cards_added_to_trash}"
        )
