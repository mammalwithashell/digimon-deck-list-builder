"""Behavioral tests for BT24-051 Merukimon (Lv.6 Green/Red Digimon).

Card text:
  [Digivolve] Lv.5 w/[Beastkin]/[TS] trait: Cost 3
  When this card would be played, if there are 3 or more Digimon, reduce the
  play cost by 5.
  [On Play] [When Digivolving] Suspend 2 of your opponent's Digimon or Tamers.
  Then, 1 of your Digimon may get +5000 DP for the turn and attack your
  opponent's Digimon.
  [When Digivolving] [When Attacking] [Once Per Turn] 1 of your Digimon may
  unsuspend.
  [Your Turn] All of your [Iliad] trait Digimon gain <Rush> and <Piercing>.
"""

import pytest


@pytest.mark.behavioral
class TestBT24051Merukimon:
    """Tests for BT24-051 Merukimon."""

    def test_on_play_suspends_two_opponent_permanents(self, debug_runner):
        """On Play should suspend 2 of opponent's Digimon or Tamers."""
        runner = debug_runner(initial_memory=15)

        # Need 3+ Digimon total for cost reduction
        runner.place_on_field(1, ["BT1-010"])
        runner.place_on_field(2, ["BT1-010"])
        runner.place_on_field(2, ["BT1-010"])
        runner.place_on_field(2, ["BT2-038"])
        runner.place_on_field(2, ["BT2-038"])

        runner.inject_card(1, "BT24-051", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Merukimon")
        assert play is not None, (
            f"Should be able to play Merukimon. Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        suspended_count = sum(1 for s in snap.p2_field if s.is_suspended)
        assert suspended_count >= 2, (
            f"Expected >=2 suspended opponent permanents, got {suspended_count}. "
            f"P2 field: {[(s.card_id, s.is_suspended) for s in snap.p2_field]}")

    def test_when_attacking_unsuspend_selection_appears(self, debug_runner):
        """[When Attacking] should create an unsuspend selection when Merukimon attacks."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["BT24-051"])
        perm = runner.place_on_field(1, ["BT1-010"])
        perm.suspend()

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Merukimon")
        assert attack is not None
        runner.execute(attack)

        # The WA effect should create an unsuspend selection phase
        ps = getattr(runner.game, 'pending_selection', None)
        assert ps is not None, "WA effect should create a pending selection"
        prompt = getattr(ps, 'prompt', '')
        assert 'unsuspend' in prompt.lower(), (
            f"Expected unsuspend selection, got: {prompt}")

        # Pick the non-Merukimon target to unsuspend (action 101 = slot 1)
        actions = runner.actions()
        # Find an action that targets the suspended Digimon (not Decline)
        unsuspend_action = None
        for aid, desc in actions.items():
            if aid != 62 and 'target' in desc.lower():
                unsuspend_action = aid
        if unsuspend_action:
            runner.execute(unsuspend_action)
            runner.auto_resolve()

        snap = runner.snapshot()
        # At least one non-Merukimon Digimon should be unsuspended
        non_merukimon = [s for s in snap.p1_field if s.card_id != "BT24-051"]
        assert any(not s.is_suspended for s in non_merukimon), (
            f"A Digimon should have been unsuspended. "
            f"P1 field: {[(s.card_id, s.is_suspended) for s in snap.p1_field]}")

    def test_iliad_rush_aura(self, debug_runner):
        """[Your Turn] Iliad trait Digimon should gain Rush."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["BT24-051"])
        # BT24-040 Venusmon has Iliad trait; turn_played=1 = this turn (sickness)
        runner.place_on_field(1, ["BT24-040"], turn_played=1)

        runner.set_phase("Main")

        # With Rush from Merukimon's aura, Venusmon should be able to attack
        # despite having summoning sickness
        attack = runner.find_action("Attack player with Venusmon")
        assert attack is not None, (
            "Iliad Digimon should have Rush from Merukimon's aura. "
            f"Actions: {runner.actions()}")

    def test_iliad_non_iliad_no_rush(self, debug_runner):
        """Non-Iliad Digimon should NOT get Rush from the aura."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["BT24-051"])
        # BT1-010 Agumon does NOT have Iliad trait; turn_played=1 = sickness
        runner.place_on_field(1, ["BT1-010"], turn_played=1)

        runner.set_phase("Main")

        # Agumon should NOT be able to attack (no Rush, has sickness)
        attack = runner.find_action("Attack player with Agumon")
        assert attack is None, (
            "Non-Iliad Digimon should NOT get Rush from Merukimon's aura")
