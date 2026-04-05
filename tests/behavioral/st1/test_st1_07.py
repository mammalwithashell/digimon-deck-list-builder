"""Behavioral tests for ST1-07 Greymon (Lv.4 Red Dinosaur, DP 4000, Cost 5).

Card text:
  Inherited Effect: <Security Attack +1>

C# reference: ChangeSelfSAttackStaticEffect(changeValue: 1, isInheritedEffect: true)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST107Greymon:
    """Tests for ST1-07 Greymon inherited Security Attack +1."""

    def test_has_inherited_security_attack_plus_1(self, debug_runner):
        """ST1-07 should have exactly one inherited effect with _security_attack_modifier = 1."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST1-07"])

        card = perm.top_card
        effects = card.effect_list(None)
        sa_effects = [e for e in effects if getattr(e, '_security_attack_modifier', 0) != 0]
        assert len(sa_effects) == 1, "Should have exactly 1 Security Attack modifier effect"
        assert sa_effects[0]._security_attack_modifier == 1
        assert sa_effects[0].is_inherited_effect, "Must be an inherited effect"

    def test_security_attack_modifier_applies_when_inherited(self, debug_runner):
        """When Greymon is a digivolution source, its SA+1 should apply to the stack."""
        runner = debug_runner(initial_memory=5)
        # Place a Lv.5 with Greymon as digivolution source
        # ST1-07 (Greymon Lv.4) under any Lv.5 Red Digimon
        perm = runner.place_on_field(1, ["ST1-07", "BT1-019"])  # BT1-019 is MetalGreymon Lv.5

        # The permanent should have inherited security attack modifier from Greymon
        card = perm.card_sources[0]  # ST1-07 at bottom
        effects = card.effect_list(None)
        sa_effects = [e for e in effects if getattr(e, '_security_attack_modifier', 0) == 1]
        assert len(sa_effects) == 1
        assert sa_effects[0].is_inherited_effect

    def test_no_non_inherited_effects(self, debug_runner):
        """ST1-07 should have NO non-inherited effects."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST1-07"])

        card = perm.top_card
        effects = card.effect_list(None)
        non_inherited = [e for e in effects if not e.is_inherited_effect]
        assert len(non_inherited) == 0, "ST1-07 should have only inherited effects"
