"""Behavioral tests for P-136 Arisa Kinosaki (Tamer, Yellow, LIBERATOR, Cost 4).

Card text:
[On Play] You may play 1 [Shoemon] from your hand without paying the cost.
[Your Turn] [Once Per Turn] When one of your Digimon digivolves into a Digimon
    with the [Puppet] trait, by suspending this Tamer, gain 1 memory.
[Security] Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestP136OnPlay:
    """Tests for [On Play] play 1 [Shoemon] from hand free."""

    def test_on_play_plays_shoemon_from_hand(self, debug_runner):
        """On Play should allow playing Shoemon from hand free."""
        runner = debug_runner(initial_memory=4)

        # Clear hand, then inject only the cards we need
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-134", "hand")  # Shoemon
        runner.inject_card(1, "P-136", "hand")  # Arisa

        runner.set_phase("Main")

        action = runner.find_action("Play Arisa")
        assert action is not None, "Should find action to play Arisa Kinosaki"

        result = runner.execute(action)

        # Should enter selection to choose Shoemon
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", (
            f"Should enter SelectTarget for Shoemon play. Phase: {snap.phase}"
        )

        # Resolve: pick Shoemon
        runner.auto_resolve()

        snap = runner.snapshot()
        # Shoemon should now be on the field (played from hand)
        shoemon_on_field = any(
            slot.card_id == "P-134" for slot in snap.p1_field
        )
        assert shoemon_on_field, "Shoemon should be played onto the field"

    def test_on_play_optional_decline(self, debug_runner):
        """The 'you may' should allow declining to play Shoemon."""
        runner = debug_runner(initial_memory=4)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-134", "hand")
        runner.inject_card(1, "P-136", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Arisa")
        assert action is not None
        result = runner.execute(action)

        snap = runner.snapshot()
        if snap.phase == "SelectTarget":
            # Find and use "Pass"/"Decline" action to decline
            pass_action = runner.find_action("pass") or runner.find_action("decline")
            if pass_action is not None:
                runner.execute(pass_action)
                runner.auto_resolve()

    def test_on_play_no_shoemon_in_hand(self, debug_runner):
        """If no Shoemon in hand, the optional effect should not create a selection."""
        runner = debug_runner(initial_memory=4)

        # Clear hand — no Shoemon
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-136", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Arisa")
        assert action is not None
        result = runner.execute(action)
        runner.auto_resolve()

        # Should resolve without selection since no valid Shoemon target
        snap = runner.snapshot()
        assert snap.phase != "SelectTarget", (
            "Without Shoemon in hand, should not enter selection"
        )

    def test_on_play_exact_name_match_shoemon(self, debug_runner):
        """Should only match exact name 'Shoemon', NOT 'ShoeShoemon'."""
        runner = debug_runner(initial_memory=4)

        # Put ShoeShoemon (P-135) in hand — should NOT be playable
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-135", "hand")  # ShoeShoemon, NOT Shoemon
        runner.inject_card(1, "P-136", "hand")

        runner.set_phase("Main")

        action = runner.find_action("Play Arisa")
        assert action is not None
        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # ShoeShoemon should NOT be played onto the field
        shoeshoemon_on_field = any(
            slot.card_id == "P-135" for slot in snap.p1_field
        )
        assert not shoeshoemon_on_field, (
            "ShoeShoemon should NOT be played — filter must be exact 'Shoemon' match"
        )


@pytest.mark.behavioral
class TestP136DigivolveObserver:
    """Tests for [Your Turn] [OPT] digivolve observer + suspend + gain 1 memory."""

    def test_digivolve_into_puppet_gains_memory(self, debug_runner):
        """When your Digimon digivolves into a Puppet, suspend tamer, gain 1 memory."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "hand")

        # Place tamer P-136 on field
        tamer_perm = runner.place_on_field(1, ["P-136"])
        # Place a Lv.3 Digimon on field as digivolve base
        base_perm = runner.place_on_field(1, ["P-134"])  # Shoemon, Lv.3 Puppet

        # Inject a Lv.4 Puppet into hand for digivolution
        runner.inject_card(1, "P-135", "hand")  # ShoeShoemon Lv.4 Puppet, evo cost 2 from Yellow Lv.3

        runner.set_phase("Main")
        mem_before = runner.game.memory

        # Digivolve
        action = runner.find_action("Digivolve ShoeShoemon")
        assert action is not None, (
            f"Should find digivolve action. Available: {runner.actions()}"
        )
        result = runner.execute(action)
        runner.auto_resolve()

        # Check tamer is suspended
        assert tamer_perm.is_suspended, "Tamer should be suspended after paying cost"

        # Check memory gained
        # Digivolve costs 2, but we gain 1 from tamer: net = memory - 2 + 1 = memory - 1
        mem_after = runner.game.memory
        mem_change = mem_after - mem_before
        assert mem_change == -1, (
            f"Memory change should be -1 (evo cost -2, tamer +1). "
            f"Before: {mem_before}, After: {mem_after}, Change: {mem_change}"
        )

    def test_digivolve_observer_only_your_turn(self, debug_runner):
        """Observer should only trigger during your turn."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "hand")

        tamer_perm = runner.place_on_field(1, ["P-136"])
        base_perm = runner.place_on_field(1, ["P-134"])
        runner.inject_card(1, "P-135", "hand")

        # Switch to opponent's turn
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        runner.set_phase("Main")

        assert not tamer_perm.is_suspended, (
            "Tamer should not be suspended — not our turn"
        )

    def test_digivolve_observer_not_when_digivolving(self, debug_runner):
        """The digivolve observer should NOT use is_when_digivolving
        (that's for effects on the digivolving Digimon itself).
        It should use _is_digivolve_observer pattern only."""
        runner = debug_runner(initial_memory=5)

        tamer_perm = runner.place_on_field(1, ["P-136"])

        # Get the tamer's effects and check the observer
        effects = tamer_perm.top_card.effect_list(EffectTiming.NoTiming)
        observer_effects = [e for e in effects if getattr(e, '_is_digivolve_observer', False)]
        assert len(observer_effects) >= 1, "Should have a digivolve observer effect"

        for effect in observer_effects:
            assert not getattr(effect, 'is_when_digivolving', False), (
                "Digivolve observer should NOT have is_when_digivolving=True"
            )

    def test_digivolve_observer_requires_puppet_trait(self, debug_runner):
        """Observer should only trigger when digivolving into a Puppet trait Digimon."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "hand")

        tamer_perm = runner.place_on_field(1, ["P-136"])
        # Place a non-Puppet Lv.3 as base
        base_perm = runner.place_on_field(1, ["ST1-03"])  # Agumon, Red Lv.3 Dragon

        # Inject a non-Puppet Lv.4 (Red) into hand
        runner.inject_card(1, "ST1-05", "hand")  # Birdramon, Red Lv.4, not Puppet

        runner.set_phase("Main")

        action = runner.find_action("Digivolve Birdramon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        # Tamer should NOT be suspended (Birdramon is not Puppet)
        assert not tamer_perm.is_suspended, (
            "Tamer should NOT suspend when digivolving into non-Puppet Digimon"
        )

    def test_digivolve_observer_once_per_turn(self, debug_runner):
        """Observer is once per turn — second digivolve should not trigger again."""
        runner = debug_runner(initial_memory=10)
        runner.clear_zone(1, "hand")

        tamer_perm = runner.place_on_field(1, ["P-136"])
        base1 = runner.place_on_field(1, ["P-134"])  # Shoemon Lv.3
        runner.inject_card(1, "P-135", "hand")  # ShoeShoemon Lv.4 Puppet

        runner.set_phase("Main")

        # First digivolve
        action = runner.find_action("Digivolve ShoeShoemon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        assert tamer_perm.is_suspended, "Should suspend after first digivolve into Puppet"
        mem_after_first = runner.game.memory

        # Unsuspend tamer for potential second trigger
        tamer_perm.unsuspend()

        # Place another Lv.3 and inject another Lv.4 Puppet
        base2 = runner.place_on_field(1, ["P-134"])
        runner.inject_card(1, "ST19-08", "hand")  # ShoeShoemon Lv.4 Puppet

        action = runner.find_action("Digivolve ShoeShoemon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        # Tamer should NOT be suspended again (OPT already used)
        assert not tamer_perm.is_suspended, (
            "OPT: tamer should NOT suspend a second time in the same turn"
        )

    def test_digivolve_observer_requires_suspend_cost(self, debug_runner):
        """If the tamer is already suspended, the observer cannot fire."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "hand")

        tamer_perm = runner.place_on_field(1, ["P-136"])
        tamer_perm.suspend()  # Pre-suspend

        base_perm = runner.place_on_field(1, ["P-134"])
        runner.inject_card(1, "P-135", "hand")

        runner.set_phase("Main")
        mem_before = runner.game.memory

        action = runner.find_action("Digivolve ShoeShoemon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        # Memory should not gain extra +1 (tamer already suspended)
        mem_after = runner.game.memory
        evo_cost = 2
        expected = mem_before - evo_cost
        assert mem_after == expected, (
            f"Already-suspended tamer should not give memory. "
            f"Expected: {expected}, Got: {mem_after}"
        )


@pytest.mark.behavioral
class TestP136Security:
    """Tests for [Security] Play this card without paying the cost."""

    def test_security_effect_has_correct_timing(self, debug_runner):
        """Security effect should use EffectTiming.SecuritySkill."""
        runner = debug_runner(initial_memory=3)

        # Create a card source to inspect effects
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-136", runner.game.player1)
        effects = cs.effect_list(EffectTiming.NoTiming)

        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) >= 1, "Should have a security effect"

        for se in security_effects:
            assert se.timing == EffectTiming.SecuritySkill, (
                f"Security effect should use SecuritySkill timing, got {se.timing}"
            )

    def test_security_effect_has_process_callback(self, debug_runner):
        """Security effect must have an on_process_callback to play the card."""
        runner = debug_runner(initial_memory=3)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-136", runner.game.player1)
        effects = cs.effect_list(EffectTiming.NoTiming)

        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        for se in security_effects:
            assert se.on_process_callback is not None, (
                "Security effect must have an on_process_callback"
            )
