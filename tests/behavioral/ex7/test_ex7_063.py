"""Behavioral tests for EX7-063 Arisa Kinosaki (Tamer).

Card text:
  [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
  [All Turns] When one of your Tokens or Digimon with the [Puppet] trait is
      deleted, by suspending this Tamer, you may play 1 level 3 Digimon card
      with the [Puppet] trait from your hand without paying the cost.
  Security Effect [Security] Play this card without paying the cost.
"""

import pytest

from digimon_gym.engine.data.card_registry import CardRegistry


# Minimal decks — EX7-063 + filler for both players
TAMER_ID = "EX7-063"        # Arisa Kinosaki (our tamer under test)
PUPPET_LV3_ID = "BT2-055"   # ToyAgumon, Lv.3 Puppet
PUPPET_LV5_ID = "BT1-038"   # Lv.5 Puppet (to be deleted as trigger)
FILLER_ID = "BT1-003"       # cheap filler


def _build_deck(extras=None):
    """50-card deck: extras + filler."""
    cards = list(extras or [])
    while len(cards) < 50:
        cards.append(FILLER_ID)
    return cards


class TestEX7063Effect0StartOfMainPhase:
    """[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory."""

    def test_gain_memory_when_opponent_has_digimon(self, debug_runner):
        """Should gain 1 memory at start of main phase when opponent has a Digimon."""
        deck1 = _build_deck([TAMER_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=3)

        # Place tamer on P1 field
        runner.place_on_field(1, [TAMER_ID])
        # Place a Digimon on P2 field (opponent)
        runner.place_on_field(2, [FILLER_ID])

        before_memory = runner.game.memory

        # Trigger start of main phase by passing turn and coming back
        # Simpler: directly fire the timing
        from digimon_gym.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase, {
            'player': runner.game.player1,
            'game': runner.game,
            'turn_player': runner.game.player1,
            'opponent_player': runner.game.player2,
        })

        after_memory = runner.game.memory
        assert after_memory == before_memory + 1, (
            f"Expected memory to increase by 1 (from {before_memory} to "
            f"{before_memory + 1}), got {after_memory}"
        )

    def test_no_memory_when_no_opponent_digimon(self, debug_runner):
        """Should NOT gain memory when opponent has no Digimon on field."""
        deck1 = _build_deck([TAMER_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=3)

        # Place tamer on P1 field
        runner.place_on_field(1, [TAMER_ID])
        # No Digimon on P2 field

        before_memory = runner.game.memory

        from digimon_gym.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase, {
            'player': runner.game.player1,
            'game': runner.game,
            'turn_player': runner.game.player1,
            'opponent_player': runner.game.player2,
        })

        after_memory = runner.game.memory
        assert after_memory == before_memory, (
            f"Memory should not change when no opponent Digimon, "
            f"got {after_memory} (was {before_memory})"
        )

    def test_no_memory_on_opponent_turn(self, debug_runner):
        """Should NOT gain memory on opponent's turn (Your Turn only)."""
        deck1 = _build_deck([TAMER_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=3)

        runner.place_on_field(1, [TAMER_ID])
        runner.place_on_field(2, [FILLER_ID])

        # Simulate opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        before_memory = runner.game.memory

        from digimon_gym.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase, {
            'player': runner.game.player1,
            'game': runner.game,
            'turn_player': runner.game.player2,
            'opponent_player': runner.game.player1,
        })

        after_memory = runner.game.memory
        assert after_memory == before_memory, (
            f"Memory should not change on opponent's turn, "
            f"got {after_memory} (was {before_memory})"
        )


class TestEX7063Effect1DeletionObserver:
    """[All Turns] When one of your Tokens or Digimon with [Puppet] is deleted,
    suspend this Tamer to play Lv.3 Puppet from hand free."""

    def test_triggers_on_puppet_deletion(self, debug_runner):
        """Should trigger when a Puppet Digimon is deleted and tamer is unsuspended."""
        deck1 = _build_deck([TAMER_ID] * 4 + [PUPPET_LV3_ID] * 4 + [PUPPET_LV5_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)

        # Place tamer on P1 field (unsuspended)
        tamer_perm = runner.place_on_field(1, [TAMER_ID])
        assert not tamer_perm.is_suspended

        # Place a Puppet Digimon on P1 field (to be deleted)
        puppet_perm = runner.place_on_field(1, [PUPPET_LV5_ID])

        # Put a Lv.3 Puppet in hand
        runner.inject_card(1, PUPPET_LV3_ID, zone="hand")

        hand_before = len(runner.game.player1.hand_cards)
        field_before = len(runner.game.player1.battle_area)

        # Delete the Puppet Digimon — should fire the deletion observer
        runner.game.player1.delete_permanent(puppet_perm)

        # Auto-resolve any selection phases (play from hand)
        runner.auto_resolve(max_steps=10)

        # Tamer should now be suspended
        assert tamer_perm.is_suspended, "Tamer should be suspended after paying cost"

        # A Lv.3 Puppet should have been played from hand
        hand_after = len(runner.game.player1.hand_cards)
        field_after = len(runner.game.player1.battle_area)

        # Field: tamer + new puppet (the deleted one is gone)
        # Hand should have one fewer card (the played Lv.3 Puppet)
        assert hand_after < hand_before, (
            f"Hand should have fewer cards after playing Puppet from hand "
            f"({hand_before} -> {hand_after})"
        )

    def test_does_not_trigger_on_non_puppet_deletion(self, debug_runner):
        """Should NOT trigger when a non-Puppet, non-Token Digimon is deleted."""
        deck1 = _build_deck([TAMER_ID] * 4 + [PUPPET_LV3_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)

        tamer_perm = runner.place_on_field(1, [TAMER_ID])
        # Place a non-Puppet Digimon
        non_puppet = runner.place_on_field(1, [FILLER_ID])

        runner.inject_card(1, PUPPET_LV3_ID, zone="hand")
        hand_before = len(runner.game.player1.hand_cards)

        # Delete the non-Puppet Digimon
        runner.game.player1.delete_permanent(non_puppet)
        runner.auto_resolve(max_steps=10)

        # Tamer should NOT be suspended
        assert not tamer_perm.is_suspended, (
            "Tamer should NOT suspend when non-Puppet/non-Token Digimon deleted"
        )

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, (
            f"Hand size should not change (was {hand_before}, got {hand_after})"
        )

    def test_does_not_trigger_when_tamer_already_suspended(self, debug_runner):
        """Should NOT trigger when tamer is already suspended."""
        deck1 = _build_deck([TAMER_ID] * 4 + [PUPPET_LV3_ID] * 4 + [PUPPET_LV5_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)

        tamer_perm = runner.place_on_field(1, [TAMER_ID], is_suspended=True)
        puppet_perm = runner.place_on_field(1, [PUPPET_LV5_ID])

        runner.inject_card(1, PUPPET_LV3_ID, zone="hand")
        hand_before = len(runner.game.player1.hand_cards)

        runner.game.player1.delete_permanent(puppet_perm)
        runner.auto_resolve(max_steps=10)

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, (
            f"Hand should not change when tamer already suspended "
            f"({hand_before} -> {hand_after})"
        )

    def test_does_not_trigger_on_opponent_puppet_deletion(self, debug_runner):
        """Should NOT trigger when OPPONENT's Puppet is deleted (must be OUR Puppet)."""
        deck1 = _build_deck([TAMER_ID] * 4 + [PUPPET_LV3_ID] * 4)
        deck2 = _build_deck([PUPPET_LV5_ID] * 4)
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)

        tamer_perm = runner.place_on_field(1, [TAMER_ID])
        opp_puppet = runner.place_on_field(2, [PUPPET_LV5_ID])

        runner.inject_card(1, PUPPET_LV3_ID, zone="hand")
        hand_before = len(runner.game.player1.hand_cards)

        # Delete opponent's Puppet
        runner.game.player2.delete_permanent(opp_puppet)
        runner.auto_resolve(max_steps=10)

        assert not tamer_perm.is_suspended, (
            "Tamer should NOT suspend when opponent's Puppet is deleted"
        )
        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before, (
            f"Hand should not change ({hand_before} -> {hand_after})"
        )

    def test_triggers_on_both_turns(self, debug_runner):
        """Should trigger on both your turn and opponent's turn ([All Turns])."""
        deck1 = _build_deck([TAMER_ID] * 4 + [PUPPET_LV3_ID] * 4 + [PUPPET_LV5_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)

        # Set to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        tamer_perm = runner.place_on_field(1, [TAMER_ID])
        puppet_perm = runner.place_on_field(1, [PUPPET_LV5_ID])
        runner.inject_card(1, PUPPET_LV3_ID, zone="hand")

        hand_before = len(runner.game.player1.hand_cards)

        runner.game.player1.delete_permanent(puppet_perm)
        runner.auto_resolve(max_steps=10)

        # Should still trigger on opponent's turn ([All Turns])
        assert tamer_perm.is_suspended, (
            "Tamer should suspend even on opponent's turn (All Turns effect)"
        )


class TestEX7063SecurityEffect:
    """Security Effect [Security] Play this card without paying the cost."""

    def test_security_effect_has_security_timing(self, debug_runner):
        """The security effect should use SecuritySkill timing."""
        CardRegistry.ensure_initialized()
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.enums import EffectTiming

        db = CardDatabase()
        cs = db.create_card_source(TAMER_ID)
        effects = cs.effect_list(EffectTiming.SecuritySkill)
        security_effects = [e for e in effects
                           if e.is_security_effect and e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) >= 1, (
            f"EX7-063 should have at least 1 SecuritySkill-timed security effect, "
            f"found {len(security_effects)}. Check that timing is set to "
            f"EffectTiming.SecuritySkill (38)."
        )
