"""Behavioral tests for EX2-067 Fire Ball (Option, Red, Cost 2).

Card text:
  [Main] Delete 1 of your opponent's Digimon with 3000 DP or less.
  If an opponent's Digimon wasn't deleted by this effect, <Draw 2>
  (Draw 2 cards from your deck.)

  Security Effect [Security] Activate this card's [Main] effects.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX2067FireBall:
    """Tests for EX2-067 Fire Ball."""

    # ── Effect structure ────────────────────────────────────────────

    def test_has_option_skill_timing(self, debug_runner):
        """Script should have an OptionSkill effect for the [Main] ability."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-067")
        effects = cs.effect_list(None)
        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_skill) >= 1, "Should have at least 1 OptionSkill effect"

    def test_has_security_skill_timing(self, debug_runner):
        """Script should have a SecuritySkill effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-067")
        effects = cs.effect_list(None)
        sec_skill = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_skill) >= 1, "Should have at least 1 SecuritySkill effect"

    # ── [Main] Delete opponent Digimon with 3000 DP or less ─────────

    def test_main_deletes_opponent_digimon_with_low_dp(self, debug_runner):
        """Playing Fire Ball should delete an opponent Digimon with DP <= 3000."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        # P1 needs a Red Digimon to play Red option; use ST1-07 (4000 DP, Red)
        runner.place_on_field(1, ["ST1-07"])
        # Place a low-DP target on P2's field: ST1-03 Agumon (2000 DP, Red)
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "EX2-067", "hand")

        assert len(runner.game.player2.battle_area) == 1

        action = runner.find_action("Fire Ball")
        assert action is not None, "Should be able to play Fire Ball"
        runner.execute(action)
        runner.auto_resolve()  # Resolve target selection

        snap = runner.snapshot()
        assert "EX2-067" in snap.p1_trash, "Fire Ball should go to trash after resolution"
        assert len(snap.p2_field) == 0, (
            "Opponent's low-DP Digimon should be deleted")
        assert "ST1-03" in snap.p2_trash, (
            "Deleted Digimon should go to trash")

    def test_main_does_not_target_digimon_above_3000_dp(self, debug_runner):
        """Fire Ball should not offer Digimon with DP > 3000 as targets."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-07"])  # Red Greymon (4000 DP) for color req
        # Place only a high-DP Digimon on opponent's field
        runner.place_on_field(2, ["ST1-07"])  # Greymon, 4000 DP - not a valid target
        runner.inject_card(1, "EX2-067", "hand")

        action = runner.find_action("Fire Ball")
        assert action is not None, "Should be able to play Fire Ball"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's Digimon should still be on field (DP too high, no valid targets)
        assert len(snap.p2_field) == 1, (
            "Opponent's 4000 DP Digimon should still be on field")

    # ── Draw 2 fallback when no deletion occurs ─────────────────────

    def test_draw_2_when_no_valid_targets(self, debug_runner):
        """If no opponent Digimon has DP <= 3000, should draw 2 cards."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-07"])  # Red Greymon for color req
        # Only high-DP Digimon on opponent field
        runner.place_on_field(2, ["ST1-07"])  # Greymon 4000 DP
        runner.inject_card(1, "EX2-067", "hand")

        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("Fire Ball")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Should draw 2 (hand_before - 1 for playing the option + 2 for draw)
        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before - 1 + 2, (
            f"Should draw 2 when no valid targets. Hand was {hand_before}, "
            f"played 1 option (-1), drew 2 (+2), expected {hand_before + 1}, got {hand_after}")

    def test_draw_2_when_opponent_field_empty(self, debug_runner):
        """If opponent has no Digimon at all, should draw 2 cards."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-07"])  # Red Greymon for color req
        # No Digimon on opponent field
        runner.inject_card(1, "EX2-067", "hand")

        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("Fire Ball")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before - 1 + 2, (
            f"Should draw 2 when opponent field is empty. Hand was {hand_before}, "
            f"expected {hand_before + 1}, got {hand_after}")

    def test_no_draw_when_digimon_deleted(self, debug_runner):
        """If an opponent's Digimon IS deleted, should NOT draw 2."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-07"])  # Red Greymon (4000 DP) for color req
        # Low DP target on P2: ST1-03 Agumon (2000 DP)
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "EX2-067", "hand")

        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("Fire Ball")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        # Should NOT draw 2: hand_before - 1 (played option) + 0 (no draw)
        assert hand_after == hand_before - 1, (
            f"Should NOT draw when a Digimon was deleted. Hand was {hand_before}, "
            f"expected {hand_before - 1}, got {hand_after}")

    # ── Deletion prevention triggers Draw 2 ─────────────────────────

    def test_draw_2_when_deletion_prevented_by_evade(self, debug_runner):
        """If deletion is attempted but prevented by Evade, should draw 2."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-07"])  # Red Greymon for color req

        # Place a low-DP Digimon on P2's field and give it Evade
        evade_perm = runner.place_on_field(2, ["ST1-03"])  # Agumon 2000 DP
        evade_perm._granted_keywords['_is_evade'] = -1  # Permanent Evade grant

        runner.inject_card(1, "EX2-067", "hand")

        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("Fire Ball")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        # Evade prevents deletion -> Digimon wasn't deleted -> should draw 2
        assert hand_after == hand_before - 1 + 2, (
            f"Should draw 2 when deletion is prevented by Evade. "
            f"Hand was {hand_before}, expected {hand_before + 1}, got {hand_after}")
        # The Digimon should still be on field (survived via Evade)
        assert len(runner.game.player2.battle_area) == 1, (
            "Evade Digimon should still be on field")
        assert runner.game.player2.battle_area[0].is_suspended, (
            "Evade Digimon should be suspended after surviving")

    # ── Security effect ─────────────────────────────────────────────

    def test_security_effect_is_flagged(self, debug_runner):
        """Security effect should be flagged as a security effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-067")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"
        sec = sec_effects[0]
        assert sec.is_security_effect, "SecuritySkill should be flagged as security effect"

    def test_security_effect_deletes_low_dp_digimon(self, debug_runner):
        """Security effect should delete low-DP Digimon (same as Main)."""
        runner = debug_runner(initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-067", runner.game.player1)
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        sec = sec_effects[0]

        # Place a low-DP Digimon on opponent (P2) field
        runner.place_on_field(2, ["ST1-03"])  # Agumon 2000 DP

        p2_field_before = len(runner.game.player2.battle_area)
        ctx = {'player': runner.game.player1, 'game': runner.game}
        sec.on_process_callback(ctx)
        runner.auto_resolve()

        p2_field_after = len(runner.game.player2.battle_area)
        assert p2_field_after < p2_field_before, (
            "Security effect should delete opponent's low-DP Digimon")

    def test_security_draws_2_when_no_targets(self, debug_runner):
        """Security effect should draw 2 if no valid targets."""
        runner = debug_runner(initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-067", runner.game.player1)
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        sec = sec_effects[0]

        # No Digimon on P2 field — or only high DP
        runner.place_on_field(2, ["ST1-07"])  # 4000 DP, not a valid target

        hand_before = len(runner.game.player1.hand_cards)
        ctx = {'player': runner.game.player1, 'game': runner.game}
        sec.on_process_callback(ctx)
        runner.auto_resolve()

        hand_after = len(runner.game.player1.hand_cards)
        assert hand_after == hand_before + 2, (
            f"Security should draw 2 when no valid targets. "
            f"Before: {hand_before}, After: {hand_after}")

    # ── DP boundary condition ───────────────────────────────────────

    def test_exactly_3000_dp_is_valid_target(self, debug_runner):
        """A Digimon with exactly 3000 DP should be a valid target (<=3000)."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-07"])  # Red Greymon for color req

        # Use GameBuilder to create a Digimon with exactly 3000 DP
        from engine_py_legacy.tests.helpers.game_builder import make_card, make_permanent
        from engine_py_legacy.engine.data.enums import CardKind, CardColor
        perm_3000 = make_permanent(
            card_id="TEST-3K", name="3000DPMon",
            dp=3000, level=3, kind=CardKind.Digimon,
            colors=[CardColor.Red], owner=runner.game.player2,
        )
        runner.game.player2.battle_area.append(perm_3000)

        runner.inject_card(1, "EX2-067", "hand")

        action = runner.find_action("Fire Ball")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # 3000 DP should be deleted
        # Check that the fake perm is no longer on P2's field
        assert perm_3000 not in runner.game.player2.battle_area, (
            "Digimon with exactly 3000 DP should be a valid delete target")
