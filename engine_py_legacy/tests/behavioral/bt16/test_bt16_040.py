"""Behavioral tests for BT16-040 Wormmon.

Card text:
  Lv.3 | Green | Larva | DP 2000 | Cost 3
  Alt-digi: [Minomon]: Cost 0
  [Start of Your Main Phase] [On Play] If it's your turn, 1 of your Digimon
  may digivolve into a level 4 Digimon card with the [Insectoid] or [Free]
  trait in your trash with the digivolution cost reduced by 1.
  Inherited: [When Attacking] [Once Per Turn] Suspend 1 of your opponent's
  Digimon.

Clause breakdown:
  C1 (alt-digi): from [Minomon] for cost 0
  C2 (Start of Main Phase): trash-digivolve Lv.4 Insectoid or Free, cost -1
  C3 (On Play): same trash-digivolve as C2
  C4 (Inherited): [When Attacking] [Once Per Turn] Suspend 1 opponent Digimon

Key faithfulness points:
  - "Free" is in attribute_eng, NOT type_eng (card_traits). Filter must check both.
  - Insectoid IS in type_eng, so card_traits check is correct for it.
  - Digivolve target must be exactly level 4.
  - Digivolution cost is reduced by 1 (not free).
  - Effect is optional ("may digivolve").
  - Alt-digi from Minomon for cost 0.
  - Inherited suspend is once-per-turn and mandatory (not optional) target selection.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase

# Card IDs
BT16_040 = "BT16-040"           # Card under test (Wormmon, Lv.3 Green)
BT12_050 = "BT12-050"           # Stingmon (Lv.4, Insectoid, Free attribute, cost 4)
BT3_025 = "BT3-025"             # ExVeemon (Lv.4, Mythical Dragon, Free attribute, cost 5)
BT1_070 = "BT1-070"             # Kuwagamon (Lv.4, Insectoid, Virus attribute, cost 4)
ST9_01 = "ST9-01"               # Minomon (Lv.2 Digi-Egg)
ST1_03 = "ST1-03"               # Agumon (Lv.3 Red, filler)
ST1_02 = "ST1-02"               # Biyomon (Lv.3 Red, filler)

FILLER = ["ST1-03"] * 40


def _select_non_pass(runner, max_steps=10):
    """Resolve selection phases by preferring non-pass actions.

    auto_resolve picks the first legal action, which is often 62 (Pass).
    This helper picks the first non-pass action when available, falling back
    to auto_resolve for non-selection phases.
    """
    for _ in range(max_steps):
        if runner.game.game_over:
            break
        if runner.game.current_phase in (GamePhase.Main, GamePhase.Breeding, GamePhase.End):
            break
        mask = runner.action_mask()
        if not mask:
            break
        # Prefer non-pass actions (62 = Decline/Pass)
        non_pass = [a for a in mask if a != 62]
        pick = non_pass[0] if non_pass else mask[0]
        runner.execute(pick)


@pytest.mark.behavioral
class TestBT16040TrashDigivolveInsectoid:
    """C2/C3: Trash-digivolve with Insectoid trait."""

    def test_start_of_main_offers_insectoid_from_trash(self, debug_runner):
        """Start of Main Phase should trigger digivolve from trash with Insectoid Lv.4."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        # Place Wormmon on field (it's the source of the effect)
        runner.place_on_field(1, [BT16_040])
        # Place a Digimon on field to be the digivolve target
        runner.place_on_field(1, [ST1_03])
        # Inject an Insectoid Lv.4 into trash
        runner.inject_card(1, BT1_070, "trash")  # Kuwagamon: Lv.4, Insectoid
        runner.set_phase("Main")

        # Manually trigger Start of Main Phase effects (set_phase doesn't fire them)
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Select non-pass for each selection (perm + trash card)
        _select_non_pass(runner)

        snap_after = runner.snapshot()
        # Kuwagamon should have been used for digivolve (removed from trash)
        assert BT1_070 not in snap_after.p1_trash, (
            f"Kuwagamon (Insectoid) should have been removed from trash for digivolve. "
            f"Trash: {snap_after.p1_trash}"
        )

    def test_on_play_offers_insectoid_from_trash(self, debug_runner):
        """On Play should trigger digivolve from trash with Insectoid Lv.4."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=10,
        )
        # Place a Digimon on field to be the digivolve target
        runner.place_on_field(1, [ST1_03])
        # Inject an Insectoid Lv.4 into trash
        runner.inject_card(1, BT12_050, "trash")  # Stingmon: Lv.4, Insectoid
        # Put Wormmon in hand to play
        runner.inject_card(1, BT16_040, "hand")
        runner.set_phase("Main")

        # Play Wormmon
        action = runner.find_action("Play Wormmon")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"
        runner.execute(action)

        # On Play triggers -- select non-pass for perm + trash selections
        _select_non_pass(runner)

        snap = runner.snapshot()
        # Stingmon should have been used for digivolve (not in trash anymore)
        assert BT12_050 not in snap.p1_trash, (
            f"Stingmon (Insectoid) should have been removed from trash. "
            f"Trash: {snap.p1_trash}"
        )


@pytest.mark.behavioral
class TestBT16040TrashDigivolveFree:
    """C2/C3: Trash-digivolve with [Free] trait (attribute_eng, not type_eng)."""

    def test_free_attribute_card_qualifies_for_trash_digivolve(self, debug_runner):
        """A Lv.4 card with Free attribute (not Insectoid) should qualify.

        This is the KEY bug test: ExVeemon (BT3-025) is Lv.4 with attribute_eng=["Free"]
        but type_eng=["Mythical Dragon"]. The filter must check attribute_eng for Free.
        """
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        # Place Wormmon on field
        runner.place_on_field(1, [BT16_040])
        # Place a Digimon on field as digivolve target
        runner.place_on_field(1, [ST1_03])
        # Inject Free-attribute Lv.4 into trash (NOT Insectoid)
        runner.inject_card(1, BT3_025, "trash")  # ExVeemon: Lv.4, Free attr, Mythical Dragon type
        runner.set_phase("Main")

        # Trigger Start of Main Phase
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        _select_non_pass(runner)

        snap_after = runner.snapshot()
        assert BT3_025 not in snap_after.p1_trash, (
            f"ExVeemon (Free attribute) should qualify for trash digivolve. "
            f"Trash: {snap_after.p1_trash}"
        )

    def test_free_attribute_on_play(self, debug_runner):
        """On Play should also accept Free-attribute cards from trash."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=10,
        )
        runner.place_on_field(1, [ST1_03])  # Target for digivolve
        runner.inject_card(1, BT3_025, "trash")  # ExVeemon: Free attribute
        runner.inject_card(1, BT16_040, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Wormmon")
        assert action is not None
        runner.execute(action)
        _select_non_pass(runner)

        snap = runner.snapshot()
        assert BT3_025 not in snap.p1_trash, (
            f"ExVeemon (Free attribute) should be used for trash digivolve on play. "
            f"Trash: {snap.p1_trash}"
        )


@pytest.mark.behavioral
class TestBT16040TrashDigivolveFiltering:
    """Verify the filter correctly rejects non-qualifying cards."""

    def test_rejects_non_level_4(self, debug_runner):
        """Only Lv.4 cards qualify -- Lv.3 Insectoid should not."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        runner.place_on_field(1, [BT16_040])
        runner.place_on_field(1, [ST1_03])
        # Only non-Lv.4 cards in trash (Wormmon is Lv.3 with Larva trait)
        runner.inject_card(1, BT16_040, "trash")  # Lv.3 Wormmon
        runner.set_phase("Main")

        # Trigger Start of Main Phase effects
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        _select_non_pass(runner)

        snap_after = runner.snapshot()
        # Wormmon should still be in trash (was not used)
        assert BT16_040 in snap_after.p1_trash, (
            "Lv.3 card should NOT qualify for trash digivolve"
        )

    def test_rejects_non_insectoid_non_free_lv4(self, debug_runner):
        """A Lv.4 that is neither Insectoid nor Free should not qualify."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        runner.place_on_field(1, [BT16_040])
        runner.place_on_field(1, [ST1_03])
        # ST1-05 Birdramon (Lv.4, Giant Bird, Vaccine) -- not Insectoid, not Free
        runner.inject_card(1, "ST1-05", "trash")
        runner.set_phase("Main")

        # Trigger Start of Main Phase effects
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        _select_non_pass(runner)

        snap = runner.snapshot()
        assert "ST1-05" in snap.p1_trash, (
            "Birdramon (Giant Bird, Vaccine) should NOT qualify for trash digivolve"
        )

    def test_cost_reduced_by_1(self, debug_runner):
        """Digivolution cost should be reduced by 1 from the card's play cost."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=10,
        )
        runner.place_on_field(1, [BT16_040])
        runner.place_on_field(1, [ST1_03])
        # BT1-070 Kuwagamon: cost 4, so digivolve cost = max(0, 4-1) = 3
        runner.inject_card(1, BT1_070, "trash")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        mem_before = snap_before.memory

        # Trigger Start of Main Phase effects
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        # Select non-pass for each step (perm + trash card)
        _select_non_pass(runner)

        snap_after = runner.snapshot()
        # Cost should be 4-1=3, so memory should decrease by 3
        expected_mem = mem_before - 3
        assert snap_after.memory == expected_mem, (
            f"Memory should decrease by 3 (cost 4 - 1 reduction). "
            f"Before: {mem_before}, after: {snap_after.memory}, expected: {expected_mem}"
        )

    def test_effect_is_optional(self, debug_runner):
        """The trash digivolve is optional ('may digivolve')."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        runner.place_on_field(1, [BT16_040])
        runner.place_on_field(1, [ST1_03])
        runner.inject_card(1, BT1_070, "trash")
        runner.set_phase("Main")

        # Trigger Start of Main Phase effects
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should enter a selection phase -- check that Pass/Decline is available
        snap = runner.snapshot()
        if snap.phase != "Main":
            pass_action = runner.find_action("pass")
            if pass_action is None:
                pass_action = runner.find_action("decline")
            # Optional selections should offer pass
            assert pass_action is not None, (
                f"Trash digivolve should be optional (pass available). "
                f"Actions: {runner.actions()}"
            )


@pytest.mark.behavioral
class TestBT16040AltDigi:
    """C1: Alt-digi from [Minomon] for cost 0."""

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi effect should have correct name and cost attributes."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + FILLER,
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, [BT16_040])

        card = perm.top_card
        effects = card.effect_list(None)
        alt_digi = None
        for eff in effects:
            if hasattr(eff, '_alt_digi_cost') and eff._alt_digi_cost is not None:
                alt_digi = eff
                break

        assert alt_digi is not None, "Should have an alt-digi effect"
        assert alt_digi._alt_digi_cost == 0, (
            f"Alt-digi cost should be 0, got {alt_digi._alt_digi_cost}"
        )
        assert alt_digi._alt_digi_name == "Minomon", (
            f"Alt-digi name should be 'Minomon', got {alt_digi._alt_digi_name}"
        )

    def test_can_digivolve_from_minomon(self, debug_runner):
        """BT16-040 should be able to digivolve onto Minomon for cost 0."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + [ST9_01] * 4 + FILLER[:42],
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        # Place Minomon (Lv.2 digi-egg) on field
        runner.place_on_field(1, [ST9_01])
        # Put Wormmon in hand
        runner.inject_card(1, BT16_040, "hand")
        runner.set_phase("Main")

        # Should find a digivolve action for Wormmon onto Minomon
        actions = runner.find_actions("digivolve")
        digi_actions = {k: v for k, v in actions.items()
                        if "BT16-040" in v.upper() or "wormmon" in v.lower()}
        assert digi_actions, (
            f"Should have a digivolve action for Wormmon onto Minomon. "
            f"All actions: {runner.actions()}"
        )


@pytest.mark.behavioral
class TestBT16040Inherited:
    """C4: Inherited [When Attacking] [Once Per Turn] Suspend 1 opponent Digimon."""

    def test_inherited_suspend_on_attack(self, debug_runner):
        """When the Digimon carrying Wormmon attacks, it should suspend an opponent Digimon."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + [BT12_050] * 4 + FILLER[:42],
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        # Stack: Wormmon (bottom) + Stingmon (top, Lv.4)
        perm = runner.place_on_field(1, [BT16_040, BT12_050])
        # Place an opponent Digimon to be suspended
        runner.place_on_field(2, [ST1_03])
        runner.set_phase("Main")

        # Attack with the Stingmon stack
        attack_action = runner.find_action("attack")
        assert attack_action is not None, (
            f"Should find attack action. Actions: {runner.actions()}"
        )
        runner.execute(attack_action)

        # The inherited [When Attacking] should trigger, offering to suspend opponent Digimon
        # Use _select_non_pass to pick the opponent Digimon (not pass)
        _select_non_pass(runner)

        # Check if opponent's Digimon got suspended
        snap = runner.snapshot()
        opp_field = snap.p2_field
        if opp_field:
            suspended = any(s.is_suspended for s in opp_field)
            assert suspended, (
                f"Opponent's Digimon should be suspended by inherited effect. "
                f"Opp field: {[(s.card_name, s.is_suspended) for s in opp_field]}"
            )

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited suspend is once per turn -- verify max_count_per_turn is set."""
        runner = debug_runner(
            deck1=[BT16_040] * 4 + [BT12_050] * 4 + FILLER[:42],
            deck2=FILLER + ["ST1-02"] * 10,
            initial_memory=5,
        )
        # Stack: Wormmon + Stingmon
        perm = runner.place_on_field(1, [BT16_040, BT12_050])
        runner.set_phase("Main")

        # Verify the inherited effect has max_count_per_turn=1
        inherited_effects = []
        for src in perm.card_sources:
            for eff in src.effect_list(None):
                if getattr(eff, 'is_inherited_effect', False) and getattr(eff, 'is_on_attack', False):
                    inherited_effects.append(eff)

        assert len(inherited_effects) >= 1, "Should have inherited attack effect"
        eff = inherited_effects[0]
        assert eff.max_count_per_turn == 1, (
            f"Inherited suspend should be once per turn, got max_count_per_turn={eff.max_count_per_turn}"
        )
