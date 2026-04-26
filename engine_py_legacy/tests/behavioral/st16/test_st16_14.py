"""Behavioral tests for ST16-14 Matt Ishida (Tamer, Purple, Cost 4).

Card text:
  [Start of Your Turn] If you have 2 or less memory, set it to 3.
  [All Turns] When you trash a card in your hand using one of your effects,
      by suspending this Tamer, gain 1 memory.
  [Security] Play this card without paying the cost.

C# reference: DCGO/Assets/Scripts/CardEffect/ST16/Purple/ST16_14.cs
  - Effect 0 (OnStartTurn): CardEffectFactory.SetMemoryTo3TamerEffect(card)
      CanUseCondition: IsExistOnBattleArea + IsOwnerTurn
      CanActivateCondition: IsExistOnBattleArea + MemoryForPlayer <= 2 + CanAddMemory
      Process: SetFixedMemory(3) -- SET to 3, NOT +3
  - Effect 1 (OnDiscardHand): ActivateClass with suspend cost
      CanUseCondition: IsExistOnBattleArea + CanTriggerOnTrashHand (owner's effect, owner's card)
      CanActivateCondition: IsExistOnBattleArea + CanActivateSuspendCostEffect (not suspended + can suspend)
      Process: SuspendPermanentsClass(this) then AddMemory(1)
      is_optional = true
  - Effect 2 (SecuritySkill): CardEffectFactory.PlaySelfTamerSecurityEffect(card)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestST1614SoYTMemorySet:
    """[Start of Your Turn] If you have 2 or less memory, set it to 3."""

    def test_soyt_effect_uses_on_start_turn_timing(self, debug_runner):
        """The SoYT effect MUST use OnStartTurn timing (not OnStartMainPhase).

        C# reference: called when timing == EffectTiming.OnStartTurn in CardEffects().
        This fires in the Unsuspend phase, before Draw and Main phases.
        """
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["ST16-14"])
        card = perm.top_card
        effects = card.effect_list(None)

        soyt_effects = [e for e in effects if e.timing == EffectTiming.OnStartTurn]
        assert len(soyt_effects) >= 1, (
            f"Should have at least 1 OnStartTurn effect. "
            f"Timings found: {[e.timing for e in effects]}"
        )

    def test_soyt_sets_memory_to_3_when_at_0(self, debug_runner):
        """When memory is 0 (<=2), should SET to 3."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["ST16-14"])

        runner.game.memory = 0
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Memory should be SET to 3 from 0, got {runner.game.memory}"
        )

    def test_soyt_sets_memory_to_3_when_at_1(self, debug_runner):
        """When memory is 1 (<=2), should SET to 3."""
        runner = debug_runner(initial_memory=1)
        perm = runner.place_on_field(1, ["ST16-14"])

        runner.game.memory = 1
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Memory should be SET to 3 from 1, got {runner.game.memory}"
        )

    def test_soyt_sets_memory_to_3_when_at_2(self, debug_runner):
        """When memory is exactly 2 (boundary), should SET to 3."""
        runner = debug_runner(initial_memory=2)
        perm = runner.place_on_field(1, ["ST16-14"])

        runner.game.memory = 2
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Memory should be SET to 3 from 2, got {runner.game.memory}"
        )

    def test_soyt_does_nothing_when_at_3(self, debug_runner):
        """When memory is 3 (>2), should NOT change -- not eligible."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["ST16-14"])

        runner.game.memory = 3
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Memory should stay 3 (not eligible), got {runner.game.memory}"
        )

    def test_soyt_does_nothing_when_at_5(self, debug_runner):
        """When memory is 5 (>2), should NOT change."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"])

        runner.game.memory = 5
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 5, (
            f"Memory should stay 5, got {runner.game.memory}"
        )

    def test_soyt_sets_memory_from_negative(self, debug_runner):
        """When memory is negative (<=2), should SET to 3.

        C# uses SetFixedMemory(3), not AddMemory -- absolute set.
        """
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["ST16-14"])

        runner.game.memory = -2
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Memory should be SET to 3 from -2, got {runner.game.memory}"
        )

    def test_soyt_is_set_not_add(self, debug_runner):
        """Critical: effect SETS memory to 3, not ADDS 3.

        If memory is 1, result should be 3 (not 4).
        C# uses SetFixedMemory(3).
        """
        runner = debug_runner(initial_memory=1)
        perm = runner.place_on_field(1, ["ST16-14"])

        runner.game.memory = 1
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Should SET to 3, not ADD 3. From 1 expected 3, got {runner.game.memory}"
        )

    def test_soyt_only_fires_on_owners_turn(self, debug_runner):
        """SoYT condition: IsOwnerTurn -- should not fire on opponent's turn."""
        runner = debug_runner(initial_memory=0)
        # Place on P1's field
        perm = runner.place_on_field(1, ["ST16-14"])

        # Make it P2's turn
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        runner.game.memory = 0
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 0, (
            f"Should NOT fire on opponent's turn, memory should stay 0, got {runner.game.memory}"
        )


@pytest.mark.behavioral
class TestST1614HandTrashTrigger:
    """[All Turns] When you trash a card in your hand using one of your effects,
    by suspending this Tamer, gain 1 memory."""

    def test_hand_trash_effect_uses_on_discard_hand_timing(self, debug_runner):
        """The hand-trash trigger MUST use OnDiscardHand timing.

        C# reference: timing == EffectTiming.OnDiscardHand.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"])
        card = perm.top_card
        effects = card.effect_list(None)

        discard_effects = [e for e in effects if e.timing == EffectTiming.OnDiscardHand]
        assert len(discard_effects) >= 1, (
            f"Should have at least 1 OnDiscardHand effect. "
            f"Timings found: {[e.timing for e in effects]}"
        )

    def test_hand_trash_effect_is_optional(self, debug_runner):
        """C# ActivateClass 4th param = true => optional. Player can decline."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"])
        card = perm.top_card
        effects = card.effect_list(None)

        discard_effects = [e for e in effects if e.timing == EffectTiming.OnDiscardHand]
        assert len(discard_effects) >= 1
        assert discard_effects[0].is_optional is True, (
            "OnDiscardHand effect should be optional (C# ActivateClass optional=true)"
        )

    def test_hand_trash_suspends_tamer_and_gains_1_memory(self, debug_runner):
        """Trashing a card from hand should trigger: suspend tamer + gain 1 memory.

        C# process: SuspendPermanentsClass(this) then AddMemory(1).
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"])
        p1 = runner.game.player1

        # Give P1 a card in hand to trash
        runner.inject_card(1, "ST1-02", "hand")

        assert not perm.is_suspended, "Tamer should start unsuspended"
        memory_before = runner.game.memory

        # Trash a card from hand -- fires OnDiscardHand timing
        cards_to_trash = [c for c in p1.hand_cards if c.c_entity_base.card_id == "ST1-02"]
        assert len(cards_to_trash) >= 1, "Should have ST1-02 in hand"
        p1.trash_from_hand([cards_to_trash[0]])

        # Effect fires automatically via execute_effects. Check result.
        assert perm.is_suspended, (
            "Tamer should be suspended after paying suspend cost"
        )
        assert runner.game.memory == memory_before + 1, (
            f"Should gain 1 memory. Was {memory_before}, got {runner.game.memory}"
        )

    def test_hand_trash_does_not_fire_when_already_suspended(self, debug_runner):
        """If tamer is already suspended, cannot pay suspend cost -- no trigger.

        C# CanActivateCondition: CanActivateSuspendCostEffect checks !IsSuspended.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"], is_suspended=True)
        p1 = runner.game.player1

        runner.inject_card(1, "ST1-02", "hand")
        memory_before = runner.game.memory

        cards_to_trash = [c for c in p1.hand_cards if c.c_entity_base.card_id == "ST1-02"]
        p1.trash_from_hand([cards_to_trash[0]])

        # Memory should NOT change since tamer was already suspended
        assert runner.game.memory == memory_before, (
            f"Should NOT gain memory when tamer is already suspended. "
            f"Was {memory_before}, got {runner.game.memory}"
        )

    def test_hand_trash_does_not_fire_for_opponent_trash(self, debug_runner):
        """Should only trigger for owner's effects trashing, not opponent's.

        C# CanUseCondition: CanTriggerOnTrashHand checks EffectSourceCard.Owner == card.Owner.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"])
        p2 = runner.game.player2

        # Give P2 a card to trash
        runner.inject_card(2, "ST1-02", "hand")
        memory_before = runner.game.memory

        cards_to_trash = [c for c in p2.hand_cards if c.c_entity_base.card_id == "ST1-02"]
        p2.trash_from_hand([cards_to_trash[0]])

        # P1's tamer should NOT trigger from P2's trash
        assert not perm.is_suspended, "Tamer should NOT suspend from opponent's trash"
        assert runner.game.memory == memory_before, (
            f"Should NOT gain memory from opponent's trash. "
            f"Was {memory_before}, got {runner.game.memory}"
        )

    def test_hand_trash_fires_on_both_turns(self, debug_runner):
        """[All Turns] -- should fire on both your turn and opponent's turn.

        C# CanUseCondition does NOT check IsOwnerTurn -- it's an all-turns effect.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"])
        p1 = runner.game.player1

        # Switch to P2's turn
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        runner.inject_card(1, "ST1-02", "hand")
        memory_before = runner.game.memory

        cards_to_trash = [c for c in p1.hand_cards if c.c_entity_base.card_id == "ST1-02"]
        p1.trash_from_hand([cards_to_trash[0]])

        # Should still trigger even on opponent's turn
        assert perm.is_suspended, (
            "Tamer should suspend on opponent's turn too (All Turns effect)"
        )

    def test_hand_trash_only_triggers_once_per_trash(self, debug_runner):
        """Each trash event should only trigger the suspend cost once.
        After first trigger, tamer is suspended so subsequent trashes don't trigger.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"])
        p1 = runner.game.player1

        runner.inject_card(1, "ST1-02", "hand")
        runner.inject_card(1, "ST1-03", "hand")

        memory_before = runner.game.memory

        # Trash first card
        card1 = [c for c in p1.hand_cards if c.c_entity_base.card_id == "ST1-02"]
        p1.trash_from_hand([card1[0]])

        assert perm.is_suspended, "Should suspend after first trash"
        assert runner.game.memory == memory_before + 1

        # Trash second card -- tamer already suspended, can't pay cost
        memory_after_first = runner.game.memory
        card2 = [c for c in p1.hand_cards if c.c_entity_base.card_id == "ST1-03"]
        if card2:
            p1.trash_from_hand([card2[0]])

        assert runner.game.memory == memory_after_first, (
            f"Should NOT gain additional memory when tamer is already suspended. "
            f"Was {memory_after_first}, got {runner.game.memory}"
        )


@pytest.mark.behavioral
class TestST1614SecurityEffect:
    """[Security] Play this card without paying the cost."""

    def test_security_effect_exists(self, debug_runner):
        """Should have a SecuritySkill effect."""
        runner = debug_runner(initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST16-14", runner.game.player1)
        effects = cs.effect_list(None)

        security_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) >= 1, (
            f"Should have SecuritySkill effect. "
            f"Timings found: {[e.timing for e in effects]}"
        )

    def test_security_effect_is_security_flagged(self, debug_runner):
        """Security effect must have is_security_effect = True."""
        runner = debug_runner(initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST16-14", runner.game.player1)
        effects = cs.effect_list(None)

        security_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) >= 1
        assert security_effects[0].is_security_effect is True, (
            "Security effect must have is_security_effect flag"
        )

    def test_exactly_3_effects(self, debug_runner):
        """ST16-14 should have exactly 3 effects: SoYT, OnDiscardHand, Security."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST16-14"])
        card = perm.top_card
        effects = card.effect_list(None)

        assert len(effects) == 3, (
            f"Should have exactly 3 effects (SoYT + HandTrash + Security). "
            f"Got {len(effects)}: {[(e.effect_name, e.timing) for e in effects]}"
        )
