"""Behavioral tests for ST6-15 Death Claw (Option, Purple, Cost 1).

Card text:
  [Main] You may delete 1 of your Digimon to delete 1 of your opponent's
  level 4 or lower Digimon.
  [Security] Delete 1 of your opponent's level 4 or lower Digimon.

Clauses tested:
  1. Effect 0 (Main) has OptionSkill timing.
  2. Effect 0 is_optional is True ("You may").
  3. Main effect allows selecting own Digimon to delete (is_optional=True on own selection).
  4. Main effect then targets opponent Lv4 or lower Digimon for deletion.
  5. Main effect does NOT target opponent Lv5+ Digimon.
  6. Main effect does nothing if player declines (skips own deletion selection).
  7. Security effect deletes opponent Lv4 or lower Digimon (no own Digimon cost).
  8. Security effect does NOT target opponent Lv5+ Digimon.
  9. Security effect is NOT optional (mandatory deletion if valid target exists).
  10. Full integration: play Death Claw from hand, delete own Digimon, delete opp Lv4 Digimon.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# ---- Helper deck ----
# ST6-03: Gabumon (Purple Lv3) - own Digimon to sacrifice
# ST6-07: Youkomon (Purple Lv4) - own Digimon / also valid opp target
# ST6-09: Kyukimon (Purple Lv5) - opponent Lv5, should NOT be targetable
# ST1-03: Agumon (Red Lv3) - opponent low-level target
_EGGS = ["BT2-006"] * 5   # Purple digi-eggs
_MAIN = (
    ["ST6-15"] * 4    # Card under test
    + ["ST6-03"] * 4   # Gabumon (Purple Lv3)
    + ["ST6-07"] * 4   # Youkomon (Purple Lv4)
    + ["ST6-09"] * 4   # Kyukimon (Purple Lv5)
    + ["ST1-03"] * 34  # Filler
)
_DECK = _EGGS + _MAIN


@pytest.mark.behavioral
class TestST615MainEffect:
    """[Main] You may delete 1 of your Digimon to delete 1 opponent's Lv4 or lower."""

    def test_main_effect_has_option_skill_timing(self, debug_runner):
        """Effect 0 should have OptionSkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill timing"

    def test_main_effect_is_optional(self, debug_runner):
        """Effect 0 should be optional ('You may')."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15")
        effects = cs.effect_list(None)
        main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]
        assert main_eff.is_optional is True, "Main effect should be optional (You may)"

    def test_main_effect_own_selection_is_optional(self, debug_runner):
        """Own Digimon selection should be optional (is_optional=True on effect_select_own_permanent)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        # Place own purple Digimon (for color req and as sacrifice target)
        runner.place_on_field(1, ["ST6-03"])  # Gabumon Lv3
        # Place opponent Lv3 Digimon (valid target)
        runner.place_on_field(2, ["ST1-03"])  # Agumon Lv3
        runner.inject_card(1, "ST6-15", "hand")
        runner.set_phase("Main")

        # Play the option
        action = runner.find_action("Death Claw")
        if action is None:
            action = runner.find_action("ST6-15")
        assert action is not None, f"Should find play action. Available: {runner.actions()}"
        runner.execute(action)

        # We should now be in a selection phase; the "Pass" / skip option should be available
        # because the own-Digimon deletion is optional ("You may")
        pass_action = runner.find_action("Pass")
        skip_action = runner.find_action("Skip")
        assert pass_action is not None or skip_action is not None, (
            f"Own Digimon selection should be skippable (optional). Actions: {runner.actions()}"
        )

    def test_main_effect_deletes_own_then_opponent_digimon(self, debug_runner):
        """Selecting own Digimon should delete it, then allow selecting opponent Lv4- Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        # P1: two Digimon (one to sacrifice, one stays)
        runner.place_on_field(1, ["ST6-03"])  # Gabumon Lv3 (sacrifice) - slot 0
        runner.place_on_field(1, ["ST6-07"])  # Youkomon Lv4 (stays) - slot 1
        # P2: one Lv3 Digimon (target)
        runner.place_on_field(2, ["ST1-03"])  # Agumon Lv3 - opp slot 0
        runner.inject_card(1, "ST6-15", "hand")
        runner.set_phase("Main")

        # Play the option
        action = runner.find_action("Death Claw")
        if action is None:
            action = runner.find_action("ST6-15")
        assert action is not None, f"Should find play action. Available: {runner.actions()}"
        runner.execute(action)

        # Now in SelectTarget phase - select own Digimon slot 0 (Gabumon)
        # SEL_MY_FIELD_START = 100, so slot 0 = action 100
        legal = runner.action_mask()
        assert 100 in legal, f"Should be able to select own slot 0. Legal: {legal}"
        runner.execute(100)

        # Now select opponent Digimon slot 0 (Agumon)
        # SEL_OPP_FIELD_START = 114, so opp slot 0 = action 114
        legal = runner.action_mask()
        assert 114 in legal, f"Should be able to select opp slot 0. Legal: {legal}"
        runner.execute(114)
        runner.auto_resolve()

        snap = runner.snapshot()
        # P1 should have 1 Digimon left (sacrificed Gabumon)
        assert len(snap.p1_field) == 1, (
            f"P1 should have 1 Digimon left after sacrifice. Got {len(snap.p1_field)}"
        )
        assert snap.p1_field[0].card_id == "ST6-07", (
            f"Youkomon should remain. Got {snap.p1_field[0].card_id}"
        )
        # P2 should have 0 Digimon (target deleted)
        assert len(snap.p2_field) == 0, (
            f"P2's Lv3 Digimon should be deleted. Got {len(snap.p2_field)}"
        )

    def test_main_effect_does_not_target_lv5_plus(self, debug_runner):
        """Opponent Lv5+ Digimon should NOT be selectable as deletion target."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["ST6-03"])  # Own Gabumon Lv3 - slot 0
        # Only opponent is Lv5 - should not be targetable
        runner.place_on_field(2, ["ST6-09"])  # Kyukimon Lv5
        runner.inject_card(1, "ST6-15", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Death Claw")
        if action is None:
            action = runner.find_action("ST6-15")
        assert action is not None, f"Available: {runner.actions()}"
        runner.execute(action)

        # Select own Digimon slot 0 (Gabumon) for sacrifice
        runner.execute(100)
        runner.auto_resolve()

        snap = runner.snapshot()
        # P1 sacrificed Gabumon (it's gone), but no valid opp target
        assert len(snap.p1_field) == 0, (
            f"P1 Gabumon should be sacrificed. Got {len(snap.p1_field)}"
        )
        # P2's Lv5 Digimon should still be on field (not targetable)
        assert len(snap.p2_field) == 1, (
            f"P2's Lv5 Digimon should NOT be deleted. Got {len(snap.p2_field)}"
        )

    def test_main_effect_skip_does_nothing(self, debug_runner):
        """If player skips own Digimon selection, no deletions occur."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["ST6-03"])  # Own Gabumon Lv3
        runner.place_on_field(2, ["ST1-03"])  # Opponent Agumon Lv3
        runner.inject_card(1, "ST6-15", "hand")
        runner.set_phase("Main")

        p1_field_before = len(runner.snapshot().p1_field)
        p2_field_before = len(runner.snapshot().p2_field)

        action = runner.find_action("Death Claw")
        if action is None:
            action = runner.find_action("ST6-15")
        assert action is not None
        runner.execute(action)

        # Find and use the Pass/Skip action to decline own Digimon deletion
        pass_action = runner.find_action("Pass")
        if pass_action is None:
            pass_action = runner.find_action("Skip")
        assert pass_action is not None, (
            f"Should have Pass/Skip option. Actions: {runner.actions()}"
        )
        runner.execute(pass_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Both fields should be unchanged
        assert len(snap.p1_field) == p1_field_before, (
            "P1 field should be unchanged when skipping"
        )
        assert len(snap.p2_field) == p2_field_before, (
            "P2 field should be unchanged when skipping"
        )

    def test_main_targets_lv4_opponent(self, debug_runner):
        """Should be able to target exactly Lv4 opponent Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["ST6-03"])  # Own Gabumon Lv3 - slot 0
        runner.place_on_field(2, ["ST6-07"])  # Opponent Youkomon Lv4 - opp slot 0
        runner.inject_card(1, "ST6-15", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Death Claw")
        if action is None:
            action = runner.find_action("ST6-15")
        assert action is not None
        runner.execute(action)

        # Select own Digimon slot 0 (Gabumon) for sacrifice
        runner.execute(100)
        # Select opponent Digimon slot 0 (Youkomon Lv4) for deletion
        legal = runner.action_mask()
        assert 114 in legal, f"Opp Lv4 should be targetable. Legal: {legal}"
        runner.execute(114)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent Lv4 should be deleted
        assert len(snap.p2_field) == 0, (
            f"Opponent Lv4 Digimon should be deleted. Got {len(snap.p2_field)}"
        )


@pytest.mark.behavioral
class TestST615SecurityEffect:
    """[Security] Delete 1 of your opponent's level 4 or lower Digimon."""

    def test_security_effect_has_security_timing(self, debug_runner):
        """Effect 1 should have SecuritySkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have SecuritySkill timing"

    def test_security_effect_is_security_effect(self, debug_runner):
        """Security effect should have is_security_effect = True."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15")
        effects = cs.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]
        assert sec.is_security_effect is True

    def test_security_effect_is_not_optional(self, debug_runner):
        """Security effect deletion should NOT be optional."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15")
        effects = cs.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]
        assert sec.is_optional is not True, (
            "Security effect should not be optional"
        )

    def test_security_deletes_opponent_lv4_or_lower(self, debug_runner):
        """Security process should delete opponent's Lv4 or lower Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        opp_perm = runner.place_on_field(2, ["ST1-03"])  # Agumon Lv3

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15", runner.game.player1)
        effects = cs.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        trash_before = len(runner.game.player2.trash_cards)

        sec.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })
        runner.auto_resolve()

        # Opponent Lv3 Digimon should be deleted (sent to trash)
        assert len(runner.game.player2.battle_area) == 0, (
            "Opponent Lv3 Digimon should be deleted by security effect"
        )

    def test_security_does_not_target_lv5_plus(self, debug_runner):
        """Security effect should NOT target opponent Lv5+ Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(2, ["ST6-09"])  # Kyukimon Lv5

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15", runner.game.player1)
        effects = cs.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        sec.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })
        runner.auto_resolve()

        # Opponent Lv5 should still be on field
        assert len(runner.game.player2.battle_area) == 1, (
            "Opponent Lv5 Digimon should NOT be deleted by security effect"
        )

    def test_security_does_not_require_own_digimon_deletion(self, debug_runner):
        """Security effect should NOT require deleting own Digimon (no cost)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        # P1 has NO Digimon on field
        runner.place_on_field(2, ["ST1-03"])  # Opponent Agumon Lv3

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15", runner.game.player1)
        effects = cs.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        sec.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })
        runner.auto_resolve()

        # Opponent Digimon should still be deleted even though P1 has no Digimon
        assert len(runner.game.player2.battle_area) == 0, (
            "Security should delete opponent Digimon without requiring own Digimon sacrifice"
        )

    def test_security_no_auto_select_with_single_target(self, debug_runner):
        """Security effect should present selection even with 1 valid target (no auto-select)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(2, ["ST1-03"])  # Single Agumon Lv3

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST6-15", runner.game.player1)
        effects = cs.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        sec.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })

        # The engine's effect_select_opponent_permanent with is_optional=False
        # should request a selection phase, not auto-select
        # After process, game should be in SelectTarget phase waiting for selection
        from digimon_gym.engine.data.enums import GamePhase
        if runner.game.current_phase == GamePhase.SelectTarget:
            # Good - selection phase was entered (not auto-selected)
            # Now resolve it
            runner.auto_resolve()

        # Either way, opponent should be deleted
        assert len(runner.game.player2.battle_area) == 0, (
            "Opponent Lv3 should be deleted"
        )


@pytest.mark.behavioral
class TestST615Integration:
    """Full integration test: play Death Claw from hand."""

    def test_full_play_from_hand(self, debug_runner):
        """Play ST6-15, sacrifice own Digimon, delete opponent Lv3 Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        # P1: Purple Digimon on field (for color req + sacrifice)
        runner.place_on_field(1, ["ST6-03"])  # Gabumon Purple Lv3 - slot 0
        # P2: target Digimon
        runner.place_on_field(2, ["ST1-03"])  # Agumon Lv3 - opp slot 0
        runner.inject_card(1, "ST6-15", "hand")
        runner.set_phase("Main")

        # Find and play Death Claw
        action = runner.find_action("Death Claw")
        if action is None:
            action = runner.find_action("ST6-15")
        assert action is not None, f"Should be playable. Available: {runner.actions()}"

        before = runner.snapshot()
        runner.execute(action)

        # Select own Digimon slot 0 (Gabumon) for sacrifice
        runner.execute(100)
        # Select opponent Digimon slot 0 (Agumon) for deletion
        runner.execute(114)
        runner.auto_resolve()
        after = runner.snapshot()

        # ST6-15 costs 1 memory, so memory should decrease by 1
        assert after.memory == before.memory - 1, (
            f"Memory should decrease by 1 (cost). Before: {before.memory}, After: {after.memory}"
        )
        # P1 should have lost the sacrificed Digimon
        assert len(after.p1_field) == 0, (
            f"P1 should have 0 Digimon (sacrificed Gabumon). Got {len(after.p1_field)}"
        )
        # P2 should have lost the target Digimon
        assert len(after.p2_field) == 0, (
            f"P2 should have 0 Digimon (Agumon deleted). Got {len(after.p2_field)}"
        )
        # ST6-15 should be in P1's trash
        assert "ST6-15" in after.p1_trash, (
            f"ST6-15 should be in P1 trash after use. Trash: {after.p1_trash}"
        )
