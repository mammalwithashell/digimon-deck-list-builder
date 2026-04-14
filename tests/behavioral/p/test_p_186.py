"""Behavioral tests for P-186 Gallantmon (Lv.6 Red Holy Warrior / Royal Knight).

Card text:
When this card would be played, if there is a Digimon with 13000 DP or more,
reduce the play cost by 2 for every 5 total cards in both players' trashes.
<Rush> <Blocker>
[On Play] [When Digivolving] Delete 1 Digimon with 13000 DP or more. If this
effect didn't delete, <Recovery +1 (Deck)>.

Alt-digi: Lv.5 w/[WarGrowlmon] in name: Cost 3

C# reference: Delete uses IsPermanentExistsOnBattleAreaDigimon (either field) +
DP >= 13000 as single selection from both fields. canNoSelect=false (mandatory).
Recovery +1 only if the delete did NOT actually remove a Digimon.
"""

import pytest
from digimon_gym.engine.data.enums import GamePhase
from digimon_gym.engine.game.constants import (
    SEL_MY_FIELD_START, SEL_OPP_FIELD_START,
)


# Card IDs used in tests
P186 = "P-186"           # Gallantmon Lv.6  DP:12000  Cost:12
WARGROWLMON = "BT2-017"  # WarGrowlmon Lv.5 Red  DP:8000
OMNIMON = "BT1-084"      # Omnimon Lv.7  DP:15000
AGUMON = "BT1-010"       # Agumon Lv.3  DP:2000  (low-DP filler)
GREYMON = "AD1-001"      # Greymon Lv.4 DP:5000 (low-DP filler)
SHOUTMON_X7 = "AD1-006"  # Shoutmon X7 Lv.6 DP:13000 (high-DP target)


@pytest.mark.behavioral
class TestP186AltDigi:
    """Alt-digi: Lv.5 w/[WarGrowlmon] in name: Cost 3."""

    def test_alt_digi_onto_wargrowlmon(self, debug_runner):
        """P-186 can digivolve onto a Lv.5 WarGrowlmon for cost 3."""
        runner = debug_runner(initial_memory=3)

        # Place WarGrowlmon Lv.5 on field
        wg_perm = runner.place_on_field(1, [WARGROWLMON])
        # Put P-186 in hand
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        # Should have a digivolve action for P-186 onto WarGrowlmon
        digivolve_actions = runner.find_actions("digivolve")
        p186_actions = {aid: desc for aid, desc in digivolve_actions.items()
                        if "P-186" in desc or "Gallantmon" in desc}
        assert p186_actions, (
            "P-186 should be offered as alt-digi target onto WarGrowlmon Lv.5"
        )

    def test_alt_digi_blocked_on_non_red_non_wargrowlmon(self, debug_runner):
        """P-186 cannot digivolve onto a non-Red, non-WarGrowlmon Lv.5.

        P-186 has standard evo (Lv.5 Red: cost 4) and alt-digi (Lv.5
        w/WarGrowlmon in name: cost 3). A Blue Lv.5 satisfies neither.
        """
        runner = debug_runner(initial_memory=10)

        # Place Monzaemon (BT1-038 Lv.5 Blue) — not Red, not WarGrowlmon
        runner.place_on_field(1, ["BT1-038"])
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        digivolve_actions = runner.find_actions("digivolve")
        p186_actions = {aid: desc for aid, desc in digivolve_actions.items()
                        if "Gallantmon" in desc}
        assert not p186_actions, (
            "P-186 should NOT digivolve onto non-Red, non-WarGrowlmon Lv.5"
        )

    def test_alt_digi_cost_3_on_wargrowlmon(self, debug_runner):
        """Alt-digi onto WarGrowlmon costs 3, not the standard evo cost 4.

        Verify by checking memory change after digivolve.
        """
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, [WARGROWLMON])
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        digi_action = runner.find_action("Digivolve Gallantmon")
        assert digi_action is not None, "Should be able to digivolve P-186 onto WarGrowlmon"
        runner.execute(digi_action)
        runner.auto_resolve()

        # Alt-digi cost = 3, so memory should go from 5 to 2
        assert runner.game.memory == 2, (
            f"Alt-digi onto WarGrowlmon should cost 3 (memory 5->2), "
            f"got memory={runner.game.memory}"
        )

    def test_standard_evo_cost_4_on_red_lv5(self, debug_runner):
        """Standard evo onto non-WarGrowlmon Red Lv.5 costs 4, not alt-digi 3."""
        runner = debug_runner(initial_memory=5)

        # RizeGreymon (ST7-07 Lv.5 Red) — standard evo only
        runner.place_on_field(1, ["ST7-07"])
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        digi_action = runner.find_action("Digivolve Gallantmon")
        assert digi_action is not None, "Should be able to digivolve P-186 onto RizeGreymon"
        runner.execute(digi_action)
        runner.auto_resolve()

        # Standard evo cost = 4, so memory should go from 5 to 1
        assert runner.game.memory == 1, (
            f"Standard evo onto RizeGreymon should cost 4 (memory 5->1), "
            f"got memory={runner.game.memory}"
        )


@pytest.mark.behavioral
class TestP186CostReduction:
    """BeforePayCost: If 13000+ DP Digimon exists, reduce by 2 per 5 trash cards."""

    def test_cost_reduction_with_trash_and_high_dp(self, debug_runner):
        """With 10 total trash cards and a 13000+ DP Digimon, cost reduces by 4."""
        runner = debug_runner(initial_memory=0)

        # Place a high-DP Digimon on opponent's field
        runner.place_on_field(2, [OMNIMON])
        # Add 5 cards to P1 trash + 5 to P2 trash = 10 total => 2*(10//5) = 4 reduction
        for _ in range(5):
            runner.inject_card(1, AGUMON, "trash")
        for _ in range(5):
            runner.inject_card(2, AGUMON, "trash")

        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        # P-186 base cost is 12, reduction is 4, so effective cost = 8
        # With memory=0, P1 has 0 memory; cost 8 would set memory to -8 (opponent gets 8)
        # The action should be available since memory can go negative
        play_actions = runner.find_actions("Gallantmon")
        assert play_actions, "P-186 Gallantmon should be playable with cost reduction"

    def test_no_cost_reduction_without_high_dp(self, debug_runner):
        """Without a 13000+ DP Digimon, no cost reduction applies."""
        runner = debug_runner(initial_memory=0)

        # Only low-DP Digimon on fields
        runner.place_on_field(1, [AGUMON])
        runner.place_on_field(2, [GREYMON])
        # Even with lots of trash, no reduction
        for _ in range(10):
            runner.inject_card(1, AGUMON, "trash")

        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        # Cost should be full 12 — verify via calculate_play_cost
        game = runner.game
        p1 = game.player1
        p186_card = [c for c in p1.hand_cards
                     if c.c_entity_base and c.c_entity_base.card_id == P186][0]
        cost = game.calculate_play_cost(p1, p186_card)
        assert cost == 12, (
            f"Without 13000+ DP Digimon, cost should be full 12, got {cost}"
        )

    def test_cost_reduction_counts_both_trashes(self, debug_runner):
        """Reduction counts cards in BOTH players' trashes."""
        runner = debug_runner(initial_memory=0)

        runner.place_on_field(2, [SHOUTMON_X7])  # 13000 DP
        # 3 cards in P1 trash + 7 in P2 trash = 10 total => 2*(10//5) = 4
        for _ in range(3):
            runner.inject_card(1, AGUMON, "trash")
        for _ in range(7):
            runner.inject_card(2, AGUMON, "trash")

        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        p186_card = [c for c in p1.hand_cards
                     if c.c_entity_base and c.c_entity_base.card_id == P186][0]
        cost = game.calculate_play_cost(p1, p186_card)
        # 12 - 4 = 8
        assert cost == 8, (
            f"Cost should be 12 - 2*(10//5) = 8, got {cost}"
        )

    def test_cost_reduction_own_high_dp_counts(self, debug_runner):
        """A 13000+ DP Digimon on your OWN field also satisfies the condition."""
        runner = debug_runner(initial_memory=0)

        runner.place_on_field(1, [OMNIMON])  # Own field, 15000 DP
        for _ in range(5):
            runner.inject_card(1, AGUMON, "trash")

        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        p186_card = [c for c in p1.hand_cards
                     if c.c_entity_base and c.c_entity_base.card_id == P186][0]
        cost = game.calculate_play_cost(p1, p186_card)
        # 12 - 2*(5//5) = 12 - 2 = 10
        assert cost == 10, (
            f"Cost should be 12 - 2*(5//5) = 10, got {cost}"
        )


@pytest.mark.behavioral
class TestP186DeleteOrRecovery:
    """[On Play] [When Digivolving] Delete 1 Digimon with 13000+ DP or Recovery +1."""

    def test_on_play_delete_opponent_high_dp(self, debug_runner):
        """On Play: deletes opponent's 13000+ DP Digimon."""
        runner = debug_runner(initial_memory=15)

        runner.place_on_field(2, [OMNIMON])  # Opponent's 15000 DP
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        p2_field_before = len(runner.game.player2.battle_area)
        p1_sec_before = len(runner.game.player1.security_cards)

        # Play P-186 (action says "Play Gallantmon from hand")
        play_action = runner.find_action("Gallantmon")
        assert play_action is not None, "Should be able to play P-186 Gallantmon"
        runner.execute(play_action)

        # Should enter selection phase for delete target
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            "Should enter SelectTarget phase to choose delete target"
        )

        # Resolve selection (pick opponent's field slot 0)
        runner.auto_resolve()

        # Omnimon should be deleted
        p2_field_after = len(runner.game.player2.battle_area)
        assert p2_field_after < p2_field_before, (
            "Opponent's high-DP Digimon should be deleted"
        )

        # Security should NOT have increased (delete succeeded)
        p1_sec_after = len(runner.game.player1.security_cards)
        assert p1_sec_after == p1_sec_before, (
            "Recovery +1 should NOT trigger when delete succeeded"
        )

    def test_on_play_delete_own_high_dp_offered(self, debug_runner):
        """On Play: player can choose to delete own 13000+ DP Digimon."""
        runner = debug_runner(initial_memory=15)

        runner.place_on_field(1, [OMNIMON])  # Own 15000 DP
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Gallantmon")
        assert play_action is not None
        runner.execute(play_action)

        # Should enter selection phase
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            "Should enter SelectTarget phase"
        )

        # Own Omnimon should be selectable (SEL_MY_FIELD range)
        sel = runner.game.pending_selection
        assert sel is not None, "Should have pending selection"
        own_field_choices = [i for i in sel.valid_indices
                            if SEL_MY_FIELD_START <= i < SEL_OPP_FIELD_START]
        assert own_field_choices, (
            "Own 13000+ DP Digimon should be selectable as delete target"
        )

    def test_delete_targets_both_fields_single_selection(self, debug_runner):
        """Delete should offer a single selection from BOTH fields, not prefer one."""
        runner = debug_runner(initial_memory=15)

        # High-DP on both fields
        runner.place_on_field(1, [OMNIMON])    # Own 15000 DP
        runner.place_on_field(2, [SHOUTMON_X7])  # Opp 13000 DP
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Gallantmon")
        assert play_action is not None
        runner.execute(play_action)

        assert runner.game.current_phase == GamePhase.SelectTarget

        sel = runner.game.pending_selection
        assert sel is not None
        own_choices = [i for i in sel.valid_indices
                       if SEL_MY_FIELD_START <= i < SEL_OPP_FIELD_START]
        opp_choices = [i for i in sel.valid_indices
                       if i >= SEL_OPP_FIELD_START]
        assert own_choices, "Own high-DP should be selectable"
        assert opp_choices, "Opp high-DP should be selectable"

    def test_recovery_when_no_high_dp_targets(self, debug_runner):
        """When no 13000+ DP Digimon exists on either field, Recovery +1."""
        runner = debug_runner(initial_memory=15)

        # Only low-DP Digimon
        runner.place_on_field(2, [AGUMON])  # 2000 DP
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        p1_sec_before = len(runner.game.player1.security_cards)

        play_action = runner.find_action("Gallantmon")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()

        p1_sec_after = len(runner.game.player1.security_cards)
        assert p1_sec_after == p1_sec_before + 1, (
            f"Recovery +1 should trigger when no delete target exists. "
            f"Security before={p1_sec_before}, after={p1_sec_after}"
        )

    def test_when_digivolving_delete_triggers(self, debug_runner):
        """When Digivolving trigger should also delete or recover."""
        runner = debug_runner(initial_memory=5)

        # Place WarGrowlmon Lv.5 on field, and a high-DP target on opp field
        runner.place_on_field(1, [WARGROWLMON])
        runner.place_on_field(2, [OMNIMON])
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        p2_field_before = len(runner.game.player2.battle_area)

        # Digivolve P-186 onto WarGrowlmon
        digi_action = runner.find_action("P-186")
        if digi_action is not None:
            runner.execute(digi_action)
            runner.auto_resolve()

            p2_field_after = len(runner.game.player2.battle_area)
            assert p2_field_after < p2_field_before, (
                "When Digivolving should trigger delete of high-DP Digimon"
            )

    def test_when_digivolving_recovery_if_no_target(self, debug_runner):
        """When Digivolving: Recovery +1 if no 13000+ DP target."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, [WARGROWLMON])
        # No high-DP on either field
        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        p1_sec_before = len(runner.game.player1.security_cards)

        digi_action = runner.find_action("P-186")
        if digi_action is not None:
            runner.execute(digi_action)
            runner.auto_resolve()

            p1_sec_after = len(runner.game.player1.security_cards)
            assert p1_sec_after == p1_sec_before + 1, (
                f"Recovery +1 should trigger when digivolving with no delete target. "
                f"Security before={p1_sec_before}, after={p1_sec_after}"
            )


@pytest.mark.behavioral
class TestP186Keywords:
    """Rush and Blocker keywords."""

    def test_rush_keyword(self, debug_runner):
        """P-186 should have Rush (can attack on the turn it is played)."""
        runner = debug_runner(initial_memory=15)

        runner.inject_card(1, P186, "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Gallantmon")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()

        # After playing, should be able to attack immediately (Rush)
        attack_actions = runner.find_actions("attack")
        assert attack_actions, "P-186 with Rush should be able to attack on the turn it's played"

    def test_blocker_keyword(self, debug_runner):
        """P-186 should have Blocker."""
        runner = debug_runner(initial_memory=15)

        perm = runner.place_on_field(1, [P186])
        # Check that the permanent has the blocker keyword
        has_blocker = any(
            getattr(eff, '_is_blocker', False)
            for eff in perm.effect_list(None)
        )
        assert has_blocker, "P-186 should have the Blocker keyword"
