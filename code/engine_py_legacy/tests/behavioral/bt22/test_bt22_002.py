"""Behavioral tests for BT22-002 Kyaromon (Lv.2 Digi-Egg).

Card text (Inherited):
[Your Turn] [Once Per Turn] When any of your Tokens or other [Puppet] trait
Digimon are deleted, <Draw 1>.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT22002Kyaromon:
    """Tests for BT22-002 Kyaromon inherited deletion observer."""

    def test_effect_is_inherited(self, debug_runner):
        """Effect should be marked as inherited."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        perm = runner.place_on_field(1, ["BT22-002", "ST19-03"])
        # BT22-002 is the bottom card (Digi-Egg), effects come from card_sources[0]
        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        bt22_effects = [e for e in effects
                        if getattr(e, 'is_inherited_effect', False)
                        and 'BT22-002' in (getattr(e, 'effect_name', '') or '')]
        assert len(bt22_effects) >= 1

    def test_effect_is_deletion_observer(self, debug_runner):
        """Effect should use _is_deletion_observer pattern."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        perm = runner.place_on_field(1, ["BT22-002", "ST19-03"])
        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        bt22_effects = [e for e in effects
                        if 'BT22-002' in (getattr(e, 'effect_name', '') or '')]
        assert any(getattr(e, '_is_deletion_observer', False) for e in bt22_effects)

    def test_once_per_turn(self, debug_runner):
        """Effect should be Once Per Turn."""
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        perm = runner.place_on_field(1, ["BT22-002", "ST19-03"])
        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        bt22_effects = [e for e in effects
                        if 'BT22-002' in (getattr(e, 'effect_name', '') or '')]
        assert any(getattr(e, 'max_count_per_turn', 0) == 1 for e in bt22_effects)
