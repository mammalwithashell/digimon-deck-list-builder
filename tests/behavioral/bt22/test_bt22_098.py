"""Behavioral tests for BT22-098 Unique Emblem: Fable Waltz.

Card text:
  [Main] You may play 1 [Shoemon] or [Arisa Kinosaki] from your hand or trash
  without paying the cost. Then, place this card in the battle area.
  [Your Turn] When any of your [Arisa Kinosaki] suspend, <Delay>
  (By trashing this card after the placing turn, activate the effect below.)
  - 1 of your [Puppet] trait Digimon may digivolve into a [Puppet] and
    [LIBERATOR] trait Digimon card in the hand with the digivolution cost
    reduced by 3.
  [Security] Activate this card's [Main] effects.
"""

import pytest


@pytest.mark.behavioral
class TestBT22098FableWaltz:
    """Tests for BT22-098 Unique Emblem: Fable Waltz."""

    # ── [Main] effect: play Shoemon or Arisa from hand or trash ──

    def test_main_plays_shoemon_from_hand(self, debug_runner):
        """Playing this option should allow playing Shoemon from hand for free."""
        runner = debug_runner(initial_memory=10)
        # Need a Yellow Digimon/Tamer on field for option color requirement
        runner.place_on_field(1, ["BT22-029"], turn_played=-1)  # Yellow Shoemon
        runner.inject_card(1, "EX7-024", "hand")  # Shoemon in hand
        runner.inject_card(1, "BT22-098", "hand")  # Fable Waltz

        runner.set_phase("Main")
        play = runner.find_action("Fable Waltz")
        assert play is not None, (
            f"Should be able to play BT22-098. Actions: {runner.actions()}"
        )
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Shoemon should now be on field (played for free)
        shoemon_on_field = any(
            s.card_id == "EX7-024" for s in snap.p1_field
        )
        assert shoemon_on_field, "Shoemon should be played to field from hand"
        # The option card should remain in battle area (Delay keeps it)
        option_on_field = any(
            s.card_id == "BT22-098" for s in snap.p1_field
        )
        assert option_on_field, "BT22-098 should remain in battle area (Delay)"

    def test_main_plays_arisa_from_hand(self, debug_runner):
        """Playing this option should allow playing Arisa Kinosaki from hand."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT22-029"], turn_played=-1)  # Yellow on field
        # Clear hand so only Arisa + option are available
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT22-088", "hand")  # Arisa Kinosaki
        runner.inject_card(1, "BT22-098", "hand")  # Fable Waltz

        runner.set_phase("Main")
        play = runner.find_action("Fable Waltz")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        arisa_on_field = any(
            s.card_id == "BT22-088" for s in snap.p1_field
        )
        assert arisa_on_field, "Arisa Kinosaki should be played to field"

    def test_main_plays_shoemon_from_trash(self, debug_runner):
        """Should allow playing Shoemon from trash."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT22-029"], turn_played=-1)  # Yellow on field
        runner.inject_card(1, "EX7-024", "trash")  # Shoemon in trash
        runner.inject_card(1, "BT22-098", "hand")

        runner.set_phase("Main")
        play = runner.find_action("Fable Waltz")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        shoemon_on_field = any(
            s.card_id == "EX7-024" for s in snap.p1_field
        )
        assert shoemon_on_field, "Shoemon should be played from trash"

    def test_main_does_not_play_shoeshoemon(self, debug_runner):
        """[Shoemon] is exact name - should NOT match ShoeShoemon."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT22-029"], turn_played=-1)  # Yellow on field
        # Clear hand to ensure only ShoeShoemon is available
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT22-032", "hand")  # ShoeShoemon (NOT Shoemon)
        runner.inject_card(1, "BT22-098", "hand")

        runner.set_phase("Main")
        play = runner.find_action("Fable Waltz")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        shoeshoemon_on_field = any(
            s.card_id == "BT22-032" for s in snap.p1_field
        )
        assert not shoeshoemon_on_field, (
            "ShoeShoemon should NOT be playable - card text says [Shoemon] (exact)"
        )

    # ── Delay trigger: when Arisa suspends ──

    def test_delay_triggers_when_arisa_suspends(self, debug_runner):
        """When Arisa Kinosaki suspends, the delay should trigger and trash the option."""
        runner = debug_runner(initial_memory=10)

        # Place Fable Waltz in battle area (simulating it was placed last turn)
        fable_perm = runner.place_on_field(1, ["BT22-098"], turn_played=-1)
        # Place Arisa Kinosaki tamer on field
        arisa_perm = runner.place_on_field(1, ["BT22-088"], turn_played=-1)

        runner.set_phase("Main")

        # Suspend Arisa to trigger the delay
        arisa_perm.suspend()

        # The delay should have trashed the option card and entered
        # a permanent selection phase. Decline the optional digivolve.
        runner.auto_resolve()

        snap = runner.snapshot()
        # The option card should have been trashed (delay activated)
        option_on_field = any(
            s.card_id == "BT22-098" for s in snap.p1_field
        )
        assert not option_on_field, (
            "BT22-098 should be trashed after delay activation"
        )

    def test_delay_does_not_trigger_for_non_arisa(self, debug_runner):
        """Suspending a non-Arisa tamer should NOT trigger the delay."""
        runner = debug_runner(initial_memory=10)

        fable_perm = runner.place_on_field(1, ["BT22-098"], turn_played=-1)
        # Place a non-Arisa tamer (Taichi Yagami, Red)
        other_tamer = runner.place_on_field(1, ["ST1-12"], turn_played=-1)

        runner.set_phase("Main")

        # Suspend the non-Arisa tamer
        other_tamer.suspend()
        runner.auto_resolve()

        snap = runner.snapshot()
        # The option card should still be in battle area
        option_on_field = any(
            s.card_id == "BT22-098" for s in snap.p1_field
        )
        assert option_on_field, (
            "BT22-098 should NOT be trashed when non-Arisa suspends"
        )

    def test_delay_digivolve_puppet_with_cost_reduction(self, debug_runner):
        """Delay should allow a Puppet Digimon to digivolve with cost reduced by 3."""
        runner = debug_runner(initial_memory=5)

        fable_perm = runner.place_on_field(1, ["BT22-098"], turn_played=-1)
        arisa_perm = runner.place_on_field(1, ["BT22-088"], turn_played=-1)
        # Shoemon Lv.3 (Puppet+LIBERATOR) on field
        puppet_perm = runner.place_on_field(1, ["BT22-029"], turn_played=-1)
        # Clear hand so only ShoeShoemon is available for digivolve
        runner.clear_zone(1, "hand")
        # ShoeShoemon Lv.4 (Puppet+LIBERATOR, evo cost 2 from Lv.3 Yellow)
        runner.inject_card(1, "BT22-032", "hand")

        runner.set_phase("Main")
        snap_before = runner.snapshot()
        mem_before = runner.game.memory

        arisa_perm.suspend()

        # After suspend, the delay fires: option is trashed, then we get a
        # permanent selection for which Puppet Digimon to digivolve.
        # Field is now: [Arisa(idx0), Shoemon(idx1)]
        # We need to select Shoemon (the Puppet Digimon).
        legal = runner.action_mask()
        select_shoemon = next((a for a in legal if a != 62), None)
        assert select_shoemon is not None, (
            f"Should be able to select Shoemon. Actions: {runner.actions()}"
        )
        runner.execute(select_shoemon)

        # Now we should get a hand selection for digivolve target (ShoeShoemon)
        # With only ShoeShoemon in hand, auto_resolve picks it
        runner.auto_resolve()

        snap = runner.snapshot()
        # ShoeShoemon should now be on field (digivolved from Shoemon)
        shoeshoemon_on_field = any(
            s.card_id == "BT22-032" for s in snap.p1_field
        )
        assert shoeshoemon_on_field, (
            "ShoeShoemon should be digivolved onto Shoemon via delay effect"
        )
        # Cost reduction of 3 applied: engine uses play cost as base for
        # effect_digivolve_from_hand. ShoeShoemon play cost 5, minus 3 = 2.
        mem_after = runner.game.memory
        assert mem_after == mem_before - 2, (
            f"Digivolve should cost 2 (play cost 5 - 3 reduction). "
            f"Memory: {mem_before} -> {mem_after}"
        )

    # ── [Security] effect: activate Main effects ──

    def test_security_activates_main_effect(self, debug_runner):
        """When revealed in security, should activate the [Main] effect (play Shoemon/Arisa)."""
        runner = debug_runner(initial_memory=10)

        # Clear P2 hand, then add only Shoemon so it's the only play target
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "BT22-029", "hand")
        # Put the option in defender's security
        runner.inject_card(2, "BT22-098", "security_top")

        # Place an attacker for player 1
        attacker = runner.place_on_field(1, ["ST1-02"], turn_played=-1)

        runner.set_phase("Main")
        attack = runner.find_action("Attack player with")
        assert attack is not None, "Should be able to attack"
        runner.execute(attack)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Shoemon should be played to P2's field from security effect
        shoemon_on_p2 = any(
            s.card_id == "BT22-029" for s in snap.p2_field
        )
        assert shoemon_on_p2, (
            "Security effect should activate [Main] and play Shoemon"
        )
