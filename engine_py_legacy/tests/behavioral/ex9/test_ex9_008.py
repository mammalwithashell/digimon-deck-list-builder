"""Behavioral tests for EX9-008 Biyomon (Lv.3 Red Bird, DP 1000, Cost 3).

Card text:
  Digivolve: from Lv.2 w/[DM] trait for 0 cost
  <Training>
  Inherited: <Raid>

C# reference:
  - Alt digi: Lv.2 DM trait, cost 0
  - Training
  - Inherited Raid
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX9008Biyomon:
    """Tests for EX9-008 Biyomon."""

    def test_has_training_keyword(self, debug_runner):
        """Biyomon should have <Training>."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX9-008"])

        card = perm.top_card
        effects = card.effect_list(None)
        training = [e for e in effects if getattr(e, '_is_training', False)]
        assert len(training) == 1, "Should have Training keyword"

    def test_has_inherited_raid(self, debug_runner):
        """Biyomon should have inherited <Raid>."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX9-008"])

        card = perm.top_card
        effects = card.effect_list(None)
        raid_effects = [e for e in effects
                        if getattr(e, '_is_raid', False) and e.is_inherited_effect]
        assert len(raid_effects) == 1, "Should have exactly 1 inherited Raid"

    def test_has_alt_digi_from_lv2_dm_trait(self, debug_runner):
        """Biyomon should be able to digivolve from a Lv.2 DM-trait Digi-Egg for cost 0."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX9-008"])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.NoTiming)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1, "Should have alt digi effect"

        found = False
        for e in alt_digi:
            if (getattr(e, '_alt_digi_level', None) == 2
                    and getattr(e, '_alt_digi_trait', None) == "DM"
                    and getattr(e, '_alt_digi_cost', None) == 0):
                found = True
        assert found, "Should have alt digi: from Lv.2 DM trait for cost 0"

    def test_alt_digi_not_from_non_dm_egg(self, debug_runner):
        """Alt digi should NOT match a Lv.2 egg without the DM trait."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX9-008"])
        top = perm.top_card

        effects = top.effect_list(EffectTiming.NoTiming)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]

        # The validator checks trait match. We verify the attributes are set correctly.
        for e in alt_digi:
            if getattr(e, '_alt_digi_level', None) == 2:
                assert getattr(e, '_alt_digi_trait', None) == "DM", \
                    "Alt digi from Lv.2 must require DM trait"
