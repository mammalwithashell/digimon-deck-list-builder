"""Behavioral tests for ST6-14 Matt Ishida (Tamer, Purple, Cost 2).

Card text:
  [Your Turn] When one of your Digimon is deleted, you may suspend this Tamer
      to gain 1 memory.
  [Security] Play this card without paying the cost.

Key behaviors:
  - Effect 0 is a deletion observer: fires when OTHER own Digimon are deleted
  - Must use _is_deletion_observer pattern (not OnDestroyedAnyone self-deletion)
  - [Your Turn] only -- does not fire on opponent's turn
  - "you may suspend" -- optional, requires tamer to be unsuspended
  - Only fires for YOUR Digimon being deleted, not opponent's
  - Cost is suspending the tamer; reward is +1 memory
  - Effect 1 is standard [Security] Play this card without paying the cost
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# ======================================================================
# Effect 0: [Your Turn] When one of your Digimon is deleted, you may
#           suspend this Tamer to gain 1 memory.
# ======================================================================


@pytest.mark.behavioral
class TestST6014DeletionObserver:
    """Tests for ST6-14 Matt Ishida deletion observer effect."""

    def test_effect_is_deletion_observer(self, debug_runner):
        """The effect must use _is_deletion_observer pattern so it fires
        via _fire_deletion_observers when OTHER permanents are deleted."""
        runner = debug_runner(initial_memory=5)
        tamer = runner.place_on_field(1, ["ST6-14"])

        card = tamer.top_card
        all_effects = card.effect_list(None)
        observers = [e for e in all_effects if getattr(e, '_is_deletion_observer', False)]
        assert len(observers) >= 1, (
            "ST6-14 must have a _is_deletion_observer effect. "
            "The 'when one of your Digimon is deleted' effect observes OTHER "
            "permanents being deleted, not self-deletion."
        )

    def test_effect_is_optional(self, debug_runner):
        """The effect says 'you may suspend' so it must be optional."""
        runner = debug_runner(initial_memory=5)
        tamer = runner.place_on_field(1, ["ST6-14"])

        card = tamer.top_card
        all_effects = card.effect_list(None)
        observers = [e for e in all_effects if getattr(e, '_is_deletion_observer', False)]
        assert len(observers) >= 1
        assert observers[0].is_optional, (
            "Effect must be optional ('you may suspend')"
        )

    def test_triggers_on_own_digimon_deletion(self, debug_runner):
        """When P1's Digimon is deleted on P1's turn with Matt Ishida on
        P1's field, the observer fires: tamer suspends, memory +1."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Matt Ishida (unsuspended) on P1's field
        tamer = runner.place_on_field(1, ["ST6-14"])
        assert not tamer.is_suspended, "Tamer should start unsuspended"

        # Place a Digimon on P1's field to delete
        digimon = runner.place_on_field(1, ["ST1-03"])

        memory_before = game.memory

        # It's P1's turn -- delete the Digimon by effect
        assert game.player1.is_my_turn, "Should be P1's turn"
        game.player1.delete_permanent(digimon, is_battle=False)

        # Matt Ishida should have suspended and gained 1 memory
        assert tamer.is_suspended, (
            "Matt Ishida should be suspended after own Digimon deletion on your turn"
        )
        assert game.memory == memory_before + 1, (
            f"Should gain 1 memory: was {memory_before}, now {game.memory}"
        )

    def test_does_not_trigger_on_opponent_turn(self, debug_runner):
        """[Your Turn] means the effect does NOT fire on opponent's turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Matt Ishida on P1's field
        tamer = runner.place_on_field(1, ["ST6-14"])
        digimon = runner.place_on_field(1, ["ST1-03"])

        # Switch to P2's turn
        game.switch_turn()
        assert not game.player1.is_my_turn, "Should be P2's turn"

        memory_before = game.memory

        # Delete P1's Digimon on P2's turn
        game.player1.delete_permanent(digimon, is_battle=False)

        # Tamer should NOT have suspended and memory should NOT change
        assert not tamer.is_suspended, (
            "Matt Ishida should NOT suspend on opponent's turn"
        )
        assert game.memory == memory_before, (
            "Should NOT gain memory on opponent's turn"
        )

    def test_does_not_trigger_when_already_suspended(self, debug_runner):
        """If the tamer is already suspended, the effect cannot fire
        (suspend is the cost, and you can't suspend an already-suspended card)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Matt Ishida already suspended
        tamer = runner.place_on_field(1, ["ST6-14"], is_suspended=True)
        assert tamer.is_suspended

        digimon = runner.place_on_field(1, ["ST1-03"])
        memory_before = game.memory

        # Delete P1's Digimon
        game.player1.delete_permanent(digimon, is_battle=False)

        # Memory should NOT change (tamer was already suspended)
        assert game.memory == memory_before, (
            "Should NOT gain memory when tamer is already suspended"
        )

    def test_does_not_trigger_for_opponent_digimon_deletion(self, debug_runner):
        """The effect says 'one of YOUR Digimon' -- must not fire when
        an opponent's Digimon is deleted."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Matt Ishida on P1's field
        tamer = runner.place_on_field(1, ["ST6-14"])

        # Place and delete an OPPONENT's Digimon
        opp_digimon = runner.place_on_field(2, ["ST1-03"])
        memory_before = game.memory

        game.player2.delete_permanent(opp_digimon, is_battle=False)

        # Tamer should NOT have suspended
        assert not tamer.is_suspended, (
            "Matt Ishida should NOT suspend when opponent's Digimon is deleted"
        )
        assert game.memory == memory_before, (
            "Should NOT gain memory when opponent's Digimon is deleted"
        )

    def test_does_not_trigger_for_tamer_deletion(self, debug_runner):
        """The effect says 'one of your Digimon' -- a tamer being deleted
        should NOT trigger it (tamers are not Digimon)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Matt Ishida on P1's field
        tamer = runner.place_on_field(1, ["ST6-14"])

        # Place ANOTHER tamer on P1's field and delete it
        other_tamer = runner.place_on_field(1, ["ST6-14"])
        memory_before = game.memory

        game.player1.delete_permanent(other_tamer, is_battle=False)

        # Original Matt should NOT have suspended
        assert not tamer.is_suspended, (
            "Matt Ishida should NOT suspend when a tamer (non-Digimon) is deleted"
        )
        assert game.memory == memory_before, (
            "Should NOT gain memory when a tamer is deleted"
        )

    def test_triggers_on_battle_deletion_too(self, debug_runner):
        """The card text does not say 'other than in battle' -- any deletion
        of your Digimon should trigger it, including battle deletion."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        tamer = runner.place_on_field(1, ["ST6-14"])
        digimon = runner.place_on_field(1, ["ST1-03"])
        memory_before = game.memory

        # Delete by battle
        game.player1.delete_permanent(digimon, is_battle=True)

        assert tamer.is_suspended, (
            "Matt Ishida should fire on battle deletion too (card text has no restriction)"
        )
        assert game.memory == memory_before + 1, (
            "Should gain 1 memory even from battle deletion"
        )

    def test_memory_gain_is_exactly_1(self, debug_runner):
        """Verify the memory gain is exactly 1, not more."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        tamer = runner.place_on_field(1, ["ST6-14"])
        digimon = runner.place_on_field(1, ["ST1-03"])

        assert game.memory == 0
        game.player1.delete_permanent(digimon, is_battle=False)

        assert game.memory == 1, (
            f"Memory gain should be exactly 1: was 0, now {game.memory}"
        )

    def test_condition_checks_deleted_permanent_context(self, debug_runner):
        """The condition function must check deleted_permanent from context
        (the field used by _fire_deletion_observers), NOT event_permanent."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        tamer = runner.place_on_field(1, ["ST6-14"])
        digimon = runner.place_on_field(1, ["ST1-03"])

        card = tamer.top_card
        all_effects = card.effect_list(None)
        observers = [e for e in all_effects if getattr(e, '_is_deletion_observer', False)]
        assert len(observers) >= 1
        observer = observers[0]

        # Context as provided by _fire_deletion_observers
        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer,
            'card': card,
            'deleted_permanent': digimon,
            'removal_cause': 'effect',
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
        }
        assert observer.can_use_condition(context), (
            "Condition should pass with proper deletion observer context"
        )


# ======================================================================
# Effect 1: [Security] Play this card without paying the cost.
# ======================================================================


@pytest.mark.behavioral
class TestST6014SecurityEffect:
    """Tests for ST6-14 Matt Ishida [Security] effect."""

    def test_security_effect_exists(self, debug_runner):
        """Should have a security effect with SecuritySkill timing."""
        runner = debug_runner(initial_memory=5)
        tamer = runner.place_on_field(1, ["ST6-14"])

        card = tamer.top_card
        all_effects = card.effect_list(None)
        security_effects = [
            e for e in all_effects
            if getattr(e, 'is_security_effect', False)
        ]
        assert len(security_effects) >= 1, (
            "ST6-14 should have a security effect"
        )

    def test_security_effect_plays_card(self, debug_runner):
        """[Security] should play this card without paying the cost."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Inject card to hand first so we can simulate security play
        runner.inject_card(1, "ST6-14", "hand")
        hand_card = game.player1.hand_cards[-1]

        card_effects = hand_card.effect_list(None)
        security_effects = [
            e for e in card_effects
            if getattr(e, 'is_security_effect', False)
        ]
        assert len(security_effects) >= 1

        sec_effect = security_effects[0]
        field_before = len(game.player1.battle_area)

        sec_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': None,
        })

        assert len(game.player1.battle_area) == field_before + 1, (
            "Security effect should play Matt Ishida onto the field"
        )

    def test_security_effect_is_free(self, debug_runner):
        """The security play should not cost any memory."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        runner.inject_card(1, "ST6-14", "hand")
        hand_card = game.player1.hand_cards[-1]

        card_effects = hand_card.effect_list(None)
        security_effects = [
            e for e in card_effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_effect = security_effects[0]

        memory_before = game.memory
        sec_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': None,
        })

        assert game.memory == memory_before, (
            f"Security play should be free: memory was {memory_before}, now {game.memory}"
        )
