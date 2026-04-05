"""Behavioral tests for BT8-097 Crimson Blaze (Option, Red, Cost 6).

Card text (from cards.json):
  Reduce the memory cost of this card in your hand by 1 for each Digimon
  your opponent has in play.
  [Main] Your opponent can't play Digimon by effects until the end of their
  turn. Delete all of your opponent's Digimon with 6000 DP or less.

  [Security] Activate this card's [Main] effects.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT8097CrimsonBlaze:
    """Tests for BT8-097 Crimson Blaze."""

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
        from digimon_gym.engine.data.card_database import CardDatabase
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

    def test_main_deletes_digimon_6000_dp_or_less(self, debug_runner):
        """Main effect should delete all opponent Digimon with 6000 DP or less."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Need a Red Digimon on P1's field to play Red option
        runner.place_on_field(1, ["ST1-03"])
        # Place opponent Digimon: ST1-03 Agumon (2000 DP), ST1-07 Greymon (4000 DP)
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

    def test_security_activates_main_effects(self, debug_runner):
        """Security effect should have a process callback that invokes main logic."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-097")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have at least 1 security effect"

        sec = sec_effects[0]
        assert sec.on_process_callback is not None, (
            "Security effect should have a process callback to activate Main effects"
        )

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT8-097")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"
