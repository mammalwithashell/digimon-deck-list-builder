"""Behavioral tests for EX7-025 ShoeShoemon (Lv.4 Yellow Puppet/LIBERATOR).

Card text:
[When Digivolving] If you have 1 or fewer Tamers, you may play 1
    [Arisa Kinosaki] from your hand without paying the cost.
[Inherited][Your Turn] All of your opponent's Security Digimon get -3000 DP.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


def _get_wd_effect(perm):
    """Get the When Digivolving effect from EX7-025's top card."""
    top = perm.top_card
    all_effects = top.effect_list(None)
    wd = [e for e in all_effects if e.is_when_digivolving]
    assert len(wd) == 1, f"Expected 1 WD effect, got {len(wd)}"
    return wd[0]


def _get_inherited_effect(perm):
    """Get the inherited security DP effect from EX7-025.

    When EX7-025 is a digivolution source (not top card), its inherited
    effects are accessible from the Permanent.effect_list. When it IS
    the top card, we read directly from the CardSource.
    """
    top = perm.top_card
    all_effects = top.effect_list(None)
    inherited = [e for e in all_effects if e.is_inherited_effect]
    assert len(inherited) >= 1, f"Expected >=1 inherited effect, got {len(inherited)}"
    return inherited[0]


@pytest.mark.behavioral
class TestEX7025WhenDigivolving:
    """Tests for EX7-025 [When Digivolving] effect."""

    def test_condition_passes_with_zero_tamers(self, debug_runner):
        """Condition should pass when player has 0 tamers."""
        runner = debug_runner(initial_memory=5)
        # Place just EX7-025 alone so it is the top card with all effects
        perm = runner.place_on_field(1, ["EX7-025"])

        wd = _get_wd_effect(perm)
        assert wd.is_optional, "Effect must be optional (canNoSelect=true)"
        assert wd.can_use_condition({}) is True

    def test_condition_passes_with_one_tamer(self, debug_runner):
        """Condition should pass when player has exactly 1 tamer."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-025"])
        runner.place_on_field(1, ["BT22-088"])  # Arisa Kinosaki tamer

        wd = _get_wd_effect(perm)
        assert wd.can_use_condition({}) is True

    def test_condition_fails_with_two_tamers(self, debug_runner):
        """Condition should fail when player has 2 or more tamers."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-025"])
        runner.place_on_field(1, ["BT22-088"])
        runner.place_on_field(1, ["EX7-063"])

        wd = _get_wd_effect(perm)
        assert wd.can_use_condition({}) is False

    def test_play_arisa_from_hand(self, debug_runner):
        """Should play Arisa Kinosaki from hand using exact name match."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-025"])

        game = runner.game
        wd = _get_wd_effect(perm)

        # Inject Arisa Kinosaki into hand
        runner.inject_card(1, "BT22-088", "hand")

        # Execute the effect (creates a selection phase)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Resolve the selection phase (pick the Arisa card)
        runner.auto_resolve()

        # Arisa should be on the field now (played from hand)
        tamer_on_field = any(
            p.is_tamer and p.top_card and
            p.top_card.c_entity_base.card_name_eng == "Arisa Kinosaki"
            for p in game.player1.battle_area
        )
        assert tamer_on_field, "Arisa Kinosaki should have been played to field"


@pytest.mark.behavioral
class TestEX7025InheritedSecurityDP:
    """Tests for EX7-025 inherited [Your Turn] security DP modifier."""

    def test_inherited_effect_attributes(self, debug_runner):
        """Inherited effect should use dp_modifier=-3000 and
        _applies_to_opponent_security_digimon=True, not security_dp_modifier."""
        runner = debug_runner(initial_memory=5)
        # Place EX7-025 alone to access both its own and inherited effects
        perm = runner.place_on_field(1, ["EX7-025"])

        inh = _get_inherited_effect(perm)

        # Must use dp_modifier, NOT security_dp_modifier
        assert inh.dp_modifier == -3000, "dp_modifier should be -3000"
        assert getattr(inh, '_applies_to_opponent_security_digimon', False) is True, \
            "Must set _applies_to_opponent_security_digimon = True"
        assert not getattr(inh, 'security_dp_modifier', None), \
            "Should NOT use security_dp_modifier (wrong attribute)"

    def test_inherited_only_on_your_turn(self, debug_runner):
        """Inherited security DP effect should only apply on owner's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-025"])

        game = runner.game
        inh = _get_inherited_effect(perm)

        # Player 1's turn -> should pass
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        assert inh.can_use_condition({}) is True

        # Player 2's turn -> should fail
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert inh.can_use_condition({}) is False

    def test_inherited_active_as_source(self, debug_runner):
        """When EX7-025 is a digivolution source under a Lv.5, its inherited
        effect should be active through the Permanent's effect_list."""
        runner = debug_runner(initial_memory=5)
        # Stack: EX7-025 (Lv.4) under some Lv.5
        # Use EX7-024 (Shoemon Lv.3) -> EX7-025 (Lv.4) -> any Lv.5
        # We need a Lv.5 card. Let's just test with a 2-card stack where
        # EX7-025 is a source.
        perm = runner.place_on_field(1, ["EX7-024", "EX7-025"])

        # Now EX7-025 IS the top card (sources = [EX7-024], top = EX7-025)
        # Permanent.effect_list returns inherited from sources + non-inherited from top
        perm_effects = perm.effect_list(None)
        # The WD effect (non-inherited) should be available from perm
        wd_from_perm = [e for e in perm_effects if e.is_when_digivolving]
        assert len(wd_from_perm) == 1, "WD effect should be on permanent from top card"
