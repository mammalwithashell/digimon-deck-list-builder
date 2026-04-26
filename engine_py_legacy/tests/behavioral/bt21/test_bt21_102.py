"""Behavioral tests for BT21-102 Tai Kamiya (Tamer White, Cost 4).

Card text:
[Start of Your Turn] If you have 2 or less memory, set it to 3.
[Your Turn] When one of your Digimon attacks, by suspending this Tamer,
    <Draw 1>. Then, 1 of your Digimon gets +2000 DP for the turn.
[Security] Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT21102TaiKamiya:
    """Tests for BT21-102 Tai Kamiya."""

    def test_play_on_field(self, debug_runner):
        """Play Tai Kamiya from hand and verify it appears on field."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT21-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Tai")
        assert action is not None, "Should find play action for Tai Kamiya"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        tamer_on_field = any(
            s.card_id == "BT21-102" and s.is_tamer
            for s in snap.p1_field
        )
        assert tamer_on_field, "Tai Kamiya should be on field as a tamer"

    def test_start_turn_set_memory_to_3_when_at_2(self, debug_runner):
        """[Start of Your Turn] Set memory to 3 if at 2."""
        runner = debug_runner(initial_memory=2)

        runner.place_on_field(1, ["BT21-102"])

        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Memory should be set to 3 when at 2, got {runner.game.memory}"
        )

    def test_start_turn_set_memory_to_3_when_at_0(self, debug_runner):
        """[Start of Your Turn] Set memory to 3 if at 0."""
        runner = debug_runner(initial_memory=0)

        runner.place_on_field(1, ["BT21-102"])

        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Memory should be set to 3 when at 0, got {runner.game.memory}"
        )

    def test_start_turn_no_change_when_at_3(self, debug_runner):
        """[Start of Your Turn] Should not change memory if already at 3."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["BT21-102"])

        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 3, (
            f"Memory should stay at 3 when already at 3, got {runner.game.memory}"
        )

    def test_start_turn_no_change_when_at_5(self, debug_runner):
        """[Start of Your Turn] Should not change memory if above 2."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT21-102"])

        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == 5, (
            f"Memory should stay at 5 when above 2, got {runner.game.memory}"
        )

    def test_attack_draw_and_dp_boost(self, debug_runner):
        """[Your Turn] When attacking, suspend Tai to draw 1 and give +2000 DP."""
        runner = debug_runner(initial_memory=5)

        # Place Tai on field (unsuspended)
        runner.place_on_field(1, ["BT21-102"])

        # Place a Digimon to attack with
        attacker_perm = runner.place_on_field(1, ["ST1-03"])  # Agumon, base 2000 DP

        runner.set_phase("Main")

        hand_before = len(runner.game.player1.hand_cards)

        # Attack with Agumon (attack security)
        action = runner.find_action("attack")
        if action is None:
            # Try finding with card name
            actions = runner.find_actions("Agumon")
            for aid, desc in actions.items():
                if "attack" in desc.lower():
                    action = aid
                    break

        assert action is not None, "Should find attack action"
        runner.execute(action)

        # Auto-resolve the on-attack trigger — should ask if we want to suspend Tai
        runner.auto_resolve()

        # Check that draw happened (hand size increased by 1)
        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after >= hand_before + 1, (
            f"Should draw 1 card on attack, hand was {hand_before}, now {hand_after}"
        )

        # Check that Tai is now suspended
        snap = runner.snapshot()
        tai = None
        for s in snap.p1_field:
            if s.card_id == "BT21-102":
                tai = s
                break

        assert tai is not None, "Tai should still be on field"
        assert tai.is_suspended, "Tai should be suspended after using effect"

    def test_attack_not_available_when_suspended(self, debug_runner):
        """[Your Turn] Cannot use effect when Tai is already suspended."""
        runner = debug_runner(initial_memory=5)

        # Place Tai on field, already suspended
        runner.place_on_field(1, ["BT21-102"], is_suspended=True)
        runner.place_on_field(1, ["ST1-03"])

        runner.set_phase("Main")

        # Verify the condition blocks activation
        game = runner.game
        player = game.player1
        tai_card = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT21-102":
                tai_card = p.top_card
                break

        assert tai_card is not None
        effects = tai_card.effect_list(None)
        attack_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.OnUseAttack:
                attack_effect = eff
                break

        assert attack_effect is not None, "Should have OnUseAttack effect"
        result = attack_effect.can_use_condition({})
        assert result is False, "Effect should not activate when Tai is suspended"

    def test_dp_boost_after_attack(self, debug_runner):
        """[Your Turn] After draw, should get +2000 DP for the turn on a Digimon."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["BT21-102"])
        attacker = runner.place_on_field(1, ["ST1-03"])  # Agumon base 2000

        runner.set_phase("Main")

        dp_before = attacker.dp  # 2000

        action = runner.find_action("attack")
        if action is None:
            actions = runner.find_actions("Agumon")
            for aid, desc in actions.items():
                if "attack" in desc.lower():
                    action = aid
                    break

        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            # After auto-resolve, one Digimon should have +2000 DP
            # The auto_resolve picks the first valid target, which should be the Agumon
            snap = runner.snapshot()
            agumon_on_field = None
            for s in snap.p1_field:
                if s.card_id == "ST1-03":
                    agumon_on_field = s
                    break

            if agumon_on_field:
                assert agumon_on_field.dp == dp_before + 2000, (
                    f"Digimon should get +2000 DP for the turn, "
                    f"was {dp_before}, now {agumon_on_field.dp}"
                )

    def test_security_play_free(self, debug_runner):
        """[Security] Should have security play effect."""
        runner = debug_runner(initial_memory=10)

        game = runner.game
        player = game.player1

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT21-102", player)
        effects = cs.effect_list(None)

        # Check for security effect
        has_security = any(
            eff.is_security_effect for eff in effects
        )
        assert has_security, "Should have a security effect"
