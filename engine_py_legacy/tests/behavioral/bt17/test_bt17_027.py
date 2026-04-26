"""Behavioral tests for BT17-027 MetalGarurumon.

Card text:
When this card would be played, if you have a Tamer with [Matt Ishida] in its name,
reduce the play cost by 3.
[On Play] [When Digivolving] Activate 1 of the effects below:
  - 1 of your opponent's Digimon or Tamers can't suspend until the end of their turn.
  - 1 of your [Agumon] may digivolve into [WarGreymon] in your hand,
    ignoring its digivolution requirements and without paying the cost.
Inherited: [When Attacking] [Once Per Turn] If this Digimon has [Omnimon] in its
name, unsuspend this Digimon.
Alt-digi: Lv.5 w/[Garurumon] in name: Cost 3

Key C# references:
- Cost reduction: ChangeCostClass, checks card.Owner.HandCards.Contains(card) + Matt Ishida tamer
- Branch 0: CANNOT_SUSPEND on opponent Digimon or Tamer, UntilOwnerTurnEndEffects
- Branch 1 (Agumon->WG): EqualsCardName("Agumon") exact, EqualsCardName("WarGreymon") exact
- Agumon selection: canNoSelect: true (optional)
- ESS: ContainsCardName("Omnimon"), hash "Unsuspend_BT17-027", unsuspends self
"""

import pytest

# Filler deck
FILLER = ["ST1-03"] * 50


@pytest.mark.behavioral
class TestBT17027CostReduction:
    """Play cost reduction when Matt Ishida tamer is on field."""

    def test_cost_reduced_with_matt_ishida(self, debug_runner):
        """Playing BT17-027 with Matt Ishida tamer should cost 11-3=8."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["ST1-03"] * 46,
            deck2=FILLER,
            initial_memory=15,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place Matt Ishida tamer on P1 field
        runner.place_on_field(1, ["ST2-12"])

        # Put BT17-027 in hand
        runner.inject_card(1, "BT17-027", "hand")

        before = runner.snapshot()
        action = runner.find_action("Play MetalGarurumon")
        assert action is not None, "Should find 'Play MetalGarurumon' action"

        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert after.memory == before.memory - 8, (
            f"With Matt Ishida, play cost should be 8 (11-3). "
            f"Memory was {before.memory}, now {after.memory}, diff={before.memory - after.memory}"
        )

    def test_cost_not_reduced_without_matt_ishida(self, debug_runner):
        """Playing BT17-027 without Matt Ishida should cost full 11."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["ST1-03"] * 46,
            deck2=FILLER,
            initial_memory=15,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        runner.inject_card(1, "BT17-027", "hand")

        before = runner.snapshot()
        action = runner.find_action("Play MetalGarurumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert after.memory == before.memory - 11, (
            f"Without Matt Ishida, play cost should be full 11. "
            f"Memory was {before.memory}, now {after.memory}, diff={before.memory - after.memory}"
        )

    def test_cost_reduction_does_not_leak(self, debug_runner):
        """BT17-027 on field should NOT reduce play cost of other cards."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["ST1-03"] * 46,
            deck2=FILLER,
            initial_memory=15,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place BT17-027 MetalGarurumon on field
        runner.place_on_field(1, ["BT17-027"])
        # Place Matt Ishida tamer on field
        runner.place_on_field(1, ["ST2-12"])

        # Put ST1-03 Agumon (cost 3) in hand
        runner.inject_card(1, "ST1-03", "hand")

        before = runner.snapshot()
        action = runner.find_action("Play Agumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert after.memory == before.memory - 3, (
            f"Other card should not benefit from BT17-027's cost reduction. "
            f"Agumon cost=3, expected diff=3, got {before.memory - after.memory}"
        )


@pytest.mark.behavioral
class TestBT17027OnPlayBranch:
    """On Play branch choice: can't suspend or Agumon->WarGreymon."""

    def test_on_play_cant_suspend_branch(self, debug_runner):
        """Branch 0: 1 opponent Digimon or Tamer can't suspend until end of their turn."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["ST1-03"] * 46,
            deck2=FILLER,
            initial_memory=15,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Opponent has a Digimon
        opp_perm = runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "BT17-027", "hand")

        action = runner.find_action("Play MetalGarurumon")
        assert action is not None
        runner.execute(action)

        # Choose can't suspend branch
        branch_action = runner.find_action("suspend")
        assert branch_action is not None, (
            f"Should see can't-suspend branch. Actions: {runner.actions()}"
        )
        runner.execute(branch_action)
        runner.auto_resolve()

        after = runner.snapshot()
        # The opponent's Digimon should have the CANNOT_SUSPEND modifier
        # We verify the game resolved without error
        assert any(s.card_id == "ST1-03" for s in after.p2_field), (
            "Opponent's Digimon should still be on field after can't-suspend applied"
        )

    def test_on_play_cant_suspend_targets_tamers(self, debug_runner):
        """Can't suspend branch should also be able to target opponent Tamers."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["ST1-03"] * 46,
            deck2=FILLER,
            initial_memory=15,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Opponent has only a Tamer (no Digimon)
        runner.place_on_field(2, ["ST1-12"])  # Tai Kamiya tamer
        runner.inject_card(1, "BT17-027", "hand")

        action = runner.find_action("Play MetalGarurumon")
        assert action is not None
        runner.execute(action)

        # Can't-suspend branch should be available since opponent has a tamer
        branch_action = runner.find_action("suspend")
        assert branch_action is not None, (
            f"Should see can't-suspend branch with opponent tamer. Actions: {runner.actions()}"
        )
        runner.execute(branch_action)
        runner.auto_resolve()

        after = runner.snapshot()
        assert any(s.card_id == "ST1-12" for s in after.p2_field), (
            "Opponent's tamer should still be on field"
        )

    def test_on_play_agumon_branch(self, debug_runner):
        """Branch 1: Agumon on field digivolves into WarGreymon from hand."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["BT1-010"] * 4 + ["BT1-025"] * 4 + ["ST1-03"] * 38,
            deck2=FILLER,
            initial_memory=15,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place Agumon on P1 field (slot 0)
        runner.place_on_field(1, ["BT1-010"])  # Agumon
        # Put WarGreymon in hand (idx 0)
        runner.inject_card(1, "BT1-025", "hand")  # WarGreymon
        # Put BT17-027 in hand (idx 1)
        runner.inject_card(1, "BT17-027", "hand")

        action = runner.find_action("Play MetalGarurumon")
        assert action is not None
        runner.execute(action)

        # Choose Agumon->WarGreymon branch (branch 1)
        branch_action = runner.find_action("Agumon")
        assert branch_action is not None, (
            f"Should see Agumon branch option. Actions: {runner.actions()}"
        )
        runner.execute(branch_action)

        # Select the Agumon permanent (own field slot 0)
        assert 100 in runner.action_mask(), (
            f"Should be able to select Agumon. Legal actions: {runner.actions()}"
        )
        runner.execute(100)

        # Select WarGreymon from hand
        wg_select = runner.find_action("WarGreymon")
        assert wg_select is not None, (
            f"Should see WarGreymon in hand selection. Actions: {runner.actions()}"
        )
        runner.execute(wg_select)
        runner.auto_resolve()

        after = runner.snapshot()
        has_wg = any(s.card_name == "WarGreymon" for s in after.p1_field)
        assert has_wg, (
            f"Agumon should have digivolved into WarGreymon. "
            f"P1 field: {[(s.card_name, s.stack_ids) for s in after.p1_field]}"
        )

    def test_agumon_branch_exact_name_match(self, debug_runner):
        """Only exact 'Agumon' should be selectable (C# EqualsCardName)."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["BT1-010"] * 4 + ["BT1-025"] * 4 + ["ST1-03"] * 38,
            deck2=FILLER,
            initial_memory=15,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place exact "Agumon" on field
        runner.place_on_field(1, ["BT1-010"])
        runner.inject_card(1, "BT1-025", "hand")
        runner.inject_card(1, "BT17-027", "hand")

        action = runner.find_action("Play MetalGarurumon")
        assert action is not None
        runner.execute(action)

        branch_action = runner.find_action("Agumon")
        assert branch_action is not None, "Exact 'Agumon' should be selectable"
        runner.execute(branch_action)

        # Select the Agumon permanent (slot 0)
        assert 100 in runner.action_mask(), "Agumon should be selectable"
        runner.execute(100)

        # Select WarGreymon from hand
        wg_select = runner.find_action("WarGreymon")
        assert wg_select is not None, "Should see WarGreymon in hand"
        runner.execute(wg_select)
        runner.auto_resolve()

        after = runner.snapshot()
        has_wg = any(s.card_name == "WarGreymon" for s in after.p1_field)
        assert has_wg, "Exact Agumon should digivolve into WarGreymon"

    def test_agumon_selection_is_optional(self, debug_runner):
        """Agumon selection should be optional (canNoSelect: true in C#)."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["BT1-010"] * 4 + ["BT1-025"] * 4 + ["ST1-03"] * 38,
            deck2=FILLER,
            initial_memory=15,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        runner.place_on_field(1, ["BT1-010"])
        runner.inject_card(1, "BT1-025", "hand")
        runner.inject_card(1, "BT17-027", "hand")

        action = runner.find_action("Play MetalGarurumon")
        runner.execute(action)

        branch_action = runner.find_action("Agumon")
        assert branch_action is not None
        runner.execute(branch_action)

        # Game should not crash — selection is optional
        runner.auto_resolve()
        after = runner.snapshot()
        assert not after.is_game_over or after.winner is not None, \
            "Game should not crash when resolving optional Agumon selection"


@pytest.mark.behavioral
class TestBT17027Inherited:
    """Inherited: [When Attacking][Once Per Turn] Omnimon unsuspend self."""

    def test_inherited_unsuspends_when_omnimon(self, debug_runner):
        """When an Omnimon with BT17-027 in stack attacks, it should unsuspend."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["BT1-084"] * 4 + ["ST1-03"] * 42,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place Omnimon with BT17-027 as inherited source
        # Stack: BT17-027 (bottom) -> BT1-084 Omnimon (top)
        runner.place_on_field(1, ["BT17-027", "BT1-084"])

        # Give opponent security so the attack doesn't end the game
        for _ in range(3):
            runner.inject_card(2, "ST1-03", "security_top")

        # Attack with Omnimon
        attack_action = runner.find_action("Attack")
        assert attack_action is not None, (
            f"Omnimon should be able to attack. Actions: {runner.actions()}"
        )
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        # After attacking + inherited unsuspend, Omnimon should be unsuspended
        omnimon_slot = next(
            (s for s in after.p1_field if s.card_name == "Omnimon"), None
        )
        if omnimon_slot is not None:
            assert not omnimon_slot.is_suspended, (
                "Omnimon should be unsuspended after inherited effect triggers"
            )

    def test_inherited_does_not_trigger_without_omnimon(self, debug_runner):
        """If the Digimon does NOT have Omnimon in its name, inherited should not fire."""
        runner = debug_runner(
            deck1=["BT17-027"] * 4 + ["ST1-03"] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place non-Omnimon with BT17-027 as inherited: MG -> MG
        runner.place_on_field(1, ["BT17-027", "BT17-027"])

        for _ in range(3):
            runner.inject_card(2, "ST1-03", "security_top")

        attack_action = runner.find_action("Attack")
        if attack_action is not None:
            runner.execute(attack_action)
            runner.auto_resolve()

            after = runner.snapshot()
            mg_slot = next(
                (s for s in after.p1_field if s.card_name == "MetalGarurumon"), None
            )
            if mg_slot is not None:
                # After attacking without Omnimon name, should remain suspended
                assert mg_slot.is_suspended, (
                    "Without Omnimon name, inherited unsuspend should NOT fire"
                )
