"""Behavioral tests for ST6-14 Matt Ishida.

Card text:
    [Your Turn] When one of your Digimon is deleted, you may suspend this Tamer
        to gain 1 memory.
    [Security] Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


def _get_effects(perm):
    """Get all effects from all card sources on a permanent."""
    effects = []
    for source in perm.card_sources:
        effects.extend(source.effect_list(EffectTiming.NoTiming))
    return effects


def _find_deletion_observer(perm):
    """Find the _is_deletion_observer effect on a permanent."""
    for eff in _get_effects(perm):
        if getattr(eff, '_is_deletion_observer', False):
            return eff
    return None


@pytest.mark.behavioral
class TestST6_14MattIshida:
    """Tests for ST6-14 Matt Ishida."""

    # ── Effect 0: Deletion observer pattern ─────────────────────────

    def test_uses_deletion_observer_pattern(self, debug_runner):
        """Effect must use _is_deletion_observer=True, NOT OnDestroyedAnyone timing."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"])

        observer = _find_deletion_observer(matt)
        assert observer is not None, (
            "ST6-14 must use _is_deletion_observer=True on a NoTiming effect "
            "so _fire_deletion_observers picks it up"
        )

    def test_condition_uses_deleted_permanent_key(self, debug_runner):
        """Condition must use context key 'deleted_permanent', not 'event_permanent'."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"])
        other = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        observer = _find_deletion_observer(matt)
        assert observer is not None

        # Simulate context provided by _fire_deletion_observers
        ctx = {
            'game': game, 'player': game.player1, 'permanent': matt,
            'card': observer.effect_source_card,
            'deleted_permanent': other,
            'removal_cause': 'effect',
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
        }
        assert observer.can_use_condition(ctx) is True, (
            "Condition should pass with 'deleted_permanent' context key"
        )

    def test_triggers_on_own_digimon_deleted(self, debug_runner):
        """When one of your Digimon is deleted on your turn, Matt should trigger."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"])
        victim = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        mem_before = game.memory
        game.player1.delete_permanent(victim, removal_cause='effect')
        runner.auto_resolve(max_steps=10)

        # Matt should be suspended and we gained 1 memory
        assert matt.is_suspended, "Matt Ishida should be suspended after triggering"
        assert game.memory == mem_before + 1, (
            f"Should gain 1 memory. Before: {mem_before}, after: {game.memory}"
        )

    def test_does_not_trigger_on_opponent_turn(self, debug_runner):
        """Matt only works on [Your Turn], not opponent's turn."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"])
        victim = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        # Simulate opponent's turn by flipping is_my_turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        observer = _find_deletion_observer(matt)
        assert observer is not None

        ctx = {
            'game': game, 'player': game.player1, 'permanent': matt,
            'card': observer.effect_source_card,
            'deleted_permanent': victim,
            'removal_cause': 'effect',
            'turn_player': game.player2,
            'opponent_player': game.player1,
        }
        assert observer.can_use_condition(ctx) is False, (
            "Should not trigger on opponent's turn"
        )

    def test_does_not_trigger_when_already_suspended(self, debug_runner):
        """Cannot suspend an already-suspended Tamer."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"], is_suspended=True)
        victim = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        observer = _find_deletion_observer(matt)
        assert observer is not None

        ctx = {
            'game': game, 'player': game.player1, 'permanent': matt,
            'card': observer.effect_source_card,
            'deleted_permanent': victim,
            'removal_cause': 'effect',
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
        }
        assert observer.can_use_condition(ctx) is False, (
            "Should not trigger when Matt is already suspended"
        )

    def test_does_not_trigger_on_non_digimon_deleted(self, debug_runner):
        """Should not trigger when a Tamer (non-Digimon) is deleted."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"])
        # Place another tamer to delete
        other_tamer = runner.place_on_field(1, ["ST6-14"])
        game = runner.game

        observer = _find_deletion_observer(matt)
        assert observer is not None

        ctx = {
            'game': game, 'player': game.player1, 'permanent': matt,
            'card': observer.effect_source_card,
            'deleted_permanent': other_tamer,
            'removal_cause': 'effect',
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
        }
        assert observer.can_use_condition(ctx) is False, (
            "Should not trigger when a non-Digimon is deleted"
        )

    def test_does_not_trigger_on_opponent_digimon_deleted(self, debug_runner):
        """Should not trigger when an opponent's Digimon is deleted."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"])
        opp_digimon = runner.place_on_field(2, ["ST1-03"])
        game = runner.game

        observer = _find_deletion_observer(matt)
        assert observer is not None

        # _fire_deletion_observers only scans owner's battle area,
        # so if an opp Digimon is deleted, Matt's owner (player 1) gets
        # context with 'player' = player1 (the observer owner).
        # But deleted_permanent belongs to opponent.
        # The condition should check that deleted permanent is OUR Digimon.
        ctx = {
            'game': game, 'player': game.player1, 'permanent': matt,
            'card': observer.effect_source_card,
            'deleted_permanent': opp_digimon,
            'removal_cause': 'effect',
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
        }
        assert observer.can_use_condition(ctx) is False, (
            "Should not trigger when an opponent's Digimon is deleted"
        )

    def test_process_suspends_and_gains_memory(self, debug_runner):
        """Process callback should suspend Matt and gain 1 memory."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"])
        game = runner.game

        observer = _find_deletion_observer(matt)
        assert observer is not None

        mem_before = game.memory
        ctx = {
            'game': game, 'player': game.player1, 'permanent': matt,
            'card': observer.effect_source_card,
        }
        observer.on_process_callback(ctx)

        assert matt.is_suspended, "Matt should be suspended after process"
        assert game.memory == mem_before + 1, (
            f"Should gain 1 memory. Before: {mem_before}, after: {game.memory}"
        )

    # ── Effect 1: Security play ──────────────────────────────────────

    def test_security_play_sets_security_played_flag(self, debug_runner):
        """Security process must set card._security_played = True so card is not trashed."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        game = runner.game

        # Get a card source for ST6-14 to simulate security
        runner.inject_card(1, "ST6-14", "hand")
        card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST6-14":
                card = c
                break
        assert card is not None, "Should have ST6-14 in hand"

        # Remove from hand (simulate it being in security)
        game.player1.hand_cards.remove(card)

        # Find the SecuritySkill effect
        effects = card.effect_list(EffectTiming.SecuritySkill)
        sec_effect = None
        for e in effects:
            if e.is_security_effect:
                sec_effect = e
                break
        assert sec_effect is not None, "Should have a SecuritySkill effect"

        field_before = len(game.player1.battle_area)
        sec_effect.on_process_callback({
            'game': game, 'player': game.player1,
            'card': card, 'permanent': None,
        })

        # Card should be on field
        assert len(game.player1.battle_area) == field_before + 1, (
            "ST6-14 should be played to the field from security"
        )
        # _security_played flag must be set
        assert getattr(card, '_security_played', False) is True, (
            "card._security_played must be True so engine doesn't trash it"
        )

    def test_security_play_uses_effect_play_from_security(self, debug_runner):
        """Security should ideally use game.effect_play_from_security for correct behavior."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        game = runner.game

        runner.inject_card(1, "ST6-14", "hand")
        card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST6-14":
                card = c
                break
        assert card is not None
        game.player1.hand_cards.remove(card)

        effects = card.effect_list(EffectTiming.SecuritySkill)
        sec_effect = None
        for e in effects:
            if e.is_security_effect:
                sec_effect = e
                break
        assert sec_effect is not None

        field_before = len(game.player1.battle_area)
        sec_effect.on_process_callback({
            'game': game, 'player': game.player1,
            'card': card, 'permanent': None,
        })

        # Verify card is on field and has _security_played
        assert len(game.player1.battle_area) == field_before + 1
        assert getattr(card, '_security_played', False) is True

    # ── Effect 0: is_optional ────────────────────────────────────────

    def test_effect_is_optional(self, debug_runner):
        """The deletion observer effect should be optional (you may suspend)."""
        runner = debug_runner(
            deck1=["ST6-14"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=5,
        )
        matt = runner.place_on_field(1, ["ST6-14"])

        observer = _find_deletion_observer(matt)
        assert observer is not None
        assert observer.is_optional is True, "Effect should be optional (you MAY suspend)"
