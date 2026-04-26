"""Behavioral tests for BT8-097 Crimson Blaze (Option, Red, Cost 6).

Card text (from cards.json):
  Reduce the memory cost of this card in your hand by 1 for each Digimon
  your opponent has in play.
  [Main] Your opponent can't play Digimon by effects until the end of their
  turn. Delete all of your opponent's Digimon with 6000 DP or less.

  [Security] Activate this card's [Main] effects.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestBT8097CrimsonBlaze:
    """Tests for BT8-097 Crimson Blaze."""

    # ── Cost Reduction (BeforePayCost) ──────────────────────────────────

    def test_cost_reduction_per_opponent_digimon(self, debug_runner):
        """Cost should be reduced by 1 per opponent Digimon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place 3 opponent Digimon
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])

        runner.inject_card(1, "BT8-097", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-097":
                cs = c
                break
        assert cs is not None

        effects = cs.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost]
        assert len(bpc) >= 1, "Should have BeforePayCost effect"

        bpc_effect = bpc[0]
        result = bpc_effect.can_use_condition({'card_source': cs})
        assert result is True, "Condition should pass when opponent has Digimon"
        assert bpc_effect.cost_reduction == 3, (
            f"Cost reduction should be 3 for 3 opponent Digimon, got {bpc_effect.cost_reduction}"
        )

    def test_cost_reduction_leak_guard(self, debug_runner):
        """BeforePayCost should only apply to THIS card (leak guard)."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "BT8-097", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-097":
                cs = c
                break

        effects = cs.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        other_cs = db.create_card_source("ST1-03", runner.game.player1)
        result = bpc.can_use_condition({'card_source': other_cs})
        assert result is False, "BeforePayCost should not apply to other cards (leak guard)"

    def test_cost_reduction_zero_when_no_opponent_digimon(self, debug_runner):
        """With no opponent Digimon, condition should return False."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.inject_card(1, "BT8-097", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-097":
                cs = c
                break

        effects = cs.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]
        result = bpc.can_use_condition({'card_source': cs})
        assert result is False, "Condition should fail when no opponent Digimon"

    def test_cost_reduction_does_not_count_tamers(self, debug_runner):
        """Cost reduction should only count Digimon, not Tamers."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(2, ["ST1-03"])   # Digimon
        runner.place_on_field(2, ["ST1-12"])   # Tamer (Tai Kamiya)

        runner.inject_card(1, "BT8-097", "hand")

        cs = None
        for c in runner.game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-097":
                cs = c
                break

        effects = cs.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]
        bpc.can_use_condition({'card_source': cs})
        assert bpc.cost_reduction == 1, (
            f"Should only count Digimon (1), not Tamers. Got {bpc.cost_reduction}"
        )

    # ── OptionSkill / Main effect timing ────────────────────────────────

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-097")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    # ── Deletion: all opponent Digimon with 6000 DP or less ─────────────

    def test_main_deletes_digimon_6000_dp_or_less(self, debug_runner):
        """Main effect should delete all opponent Digimon with 6000 DP or less."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement
        runner.place_on_field(2, ["ST1-03"])  # 2000 DP
        runner.place_on_field(2, ["ST1-07"])  # 4000 DP

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None, "Should be able to play Crimson Blaze"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"All opponent Digimon with <=6000 DP should be deleted, {len(opp_digimon)} remain"
        )

    def test_main_does_not_delete_above_6000_dp(self, debug_runner):
        """Main effect should NOT delete opponent Digimon with >6000 DP."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement
        runner.place_on_field(2, ["ST1-11"])  # WarGreymon 12000 DP
        runner.place_on_field(2, ["ST1-03"])  # Agumon 2000 DP

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 1, (
            f"Only Digimon above 6000 DP should survive, got {len(opp_digimon)}"
        )
        assert opp_digimon[0].card_id == "ST1-11", "WarGreymon (12000 DP) should survive"

    def test_main_deletes_exactly_6000_dp(self, debug_runner):
        """Digimon with exactly 6000 DP should be deleted (<=6000)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement
        runner.place_on_field(2, ["BT1-019"])  # 6000 DP exactly

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            "Digimon with exactly 6000 DP should be deleted (<=6000)"
        )

    def test_main_does_not_delete_7000_dp(self, debug_runner):
        """Digimon with 7000 DP should NOT be deleted (>6000 boundary)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement
        runner.place_on_field(2, ["BT1-021"])  # 7000 DP

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 1, "7000 DP Digimon should survive"
        assert opp_digimon[0].card_id == "BT1-021"

    def test_main_does_not_delete_tamers(self, debug_runner):
        """Main effect should only delete Digimon, not Tamers."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement
        runner.place_on_field(2, ["ST1-12"])  # Tai Kamiya tamer
        runner.place_on_field(2, ["ST1-03"])  # Agumon 2000 DP

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_tamers = [s for s in snap.p2_field if s.is_tamer]
        assert len(opp_tamers) == 1, "Opponent's Tamer should not be deleted"

    def test_main_does_not_delete_own_digimon(self, debug_runner):
        """Main effect should only delete OPPONENT's Digimon, not own."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # P1's Digimon, 2000 DP

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        p1_digimon = [s for s in snap.p1_field if s.is_digimon]
        assert len(p1_digimon) == 1, "Own Digimon should not be deleted"

    # ── Play restriction: can't play Digimon by effects ─────────────────

    def test_main_registers_cannot_play_by_effect(self, debug_runner):
        """After playing Crimson Blaze, CANNOT_PLAY_BY_EFFECT modifier should be
        registered for opponent's Digimon plays."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Check that CANNOT_PLAY_BY_EFFECT modifier is registered
        entries = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_BY_EFFECT, [])
        assert len(entries) >= 1, (
            "Should register CANNOT_PLAY_BY_EFFECT modifier after playing Crimson Blaze"
        )

    def test_play_restriction_only_blocks_digimon(self, debug_runner):
        """The restriction should only block Digimon, not Tamers or Options."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        entries = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_BY_EFFECT, [])
        assert len(entries) >= 1, "Should have CANNOT_PLAY_BY_EFFECT modifier"

        entry = entries[0]
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Should block Digimon
        digi_cs = db.create_card_source("ST1-03", runner.game.player2)
        blocked = entry.condition is None or entry.condition(None, {'card': digi_cs})
        assert blocked, "Should block Digimon plays by effect"

        # Should NOT block Tamers
        tamer_cs = db.create_card_source("ST1-12", runner.game.player2)
        tamer_blocked = entry.condition is None or entry.condition(None, {'card': tamer_cs})
        assert not tamer_blocked, "Should NOT block Tamer plays by effect"

    def test_play_restriction_has_end_of_opponent_turn_expiry(self, debug_runner):
        """The restriction should expire at end of opponent's turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        entries = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_BY_EFFECT, [])
        assert len(entries) >= 1
        assert entries[0].expiry == 'end_of_opponent_turn', (
            f"Expiry should be 'end_of_opponent_turn', got '{entries[0].expiry}'"
        )

    def test_play_restriction_has_granting_player(self, debug_runner):
        """The restriction should track granting_player for proper cleanup."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["ST1-03"])  # Red Digimon for color requirement

        runner.inject_card(1, "BT8-097", "hand")
        action = runner.find_action("Crimson Blaze")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        entries = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_BY_EFFECT, [])
        assert len(entries) >= 1
        assert entries[0].granting_player is runner.game.player1, (
            "granting_player should be P1 (the player who used Crimson Blaze)"
        )

    # ── Security effect ─────────────────────────────────────────────────

    def test_security_activates_main_effects(self, debug_runner):
        """Security effect should have a process callback that invokes main logic."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-097")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have at least 1 security effect"

        sec = sec_effects[0]
        assert sec.on_process_callback is not None, (
            "Security effect should have a process callback to activate Main effects"
        )

    def test_security_deletes_digimon(self, debug_runner):
        """Security process should also delete opponent Digimon <= 6000 DP."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(2, ["ST1-03"])  # 2000 DP
        runner.place_on_field(2, ["ST1-11"])  # 12000 DP

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-097", runner.game.player1)
        effects = cs.effect_list(None)
        sec = [e for e in effects if getattr(e, 'is_security_effect', False)][0]

        # Invoke the security process directly
        sec.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })

        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 1, (
            f"Security should delete Digimon <=6000 DP. Remaining: {len(opp_digimon)}"
        )
        assert opp_digimon[0].card_id == "ST1-11", "Only 12000 DP should survive"

    def test_security_registers_play_restriction(self, debug_runner):
        """Security process should also register the CANNOT_PLAY_BY_EFFECT modifier."""
        runner = debug_runner(initial_memory=10)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-097", runner.game.player1)
        effects = cs.effect_list(None)
        sec = [e for e in effects if getattr(e, 'is_security_effect', False)][0]

        sec.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })

        entries = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_BY_EFFECT, [])
        assert len(entries) >= 1, (
            "Security should register CANNOT_PLAY_BY_EFFECT modifier"
        )
