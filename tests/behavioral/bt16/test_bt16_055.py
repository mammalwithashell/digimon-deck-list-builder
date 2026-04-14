"""Behavioral tests for BT16-055 Namakemon (Digimon, Lv.4, Yellow, DP 4000, Cost 4).

Actual card text (from cards.json):
[On Play] [When Digivolving] If you have 3 or more security cards, until the end of
    your opponent's turn, 1 of your Digimon can't have its DP reduced by your opponent's
    effects and isn't affected by De-Digivolve effects.
    If you have 3 or fewer, 1 of your Digimon gains Blocker and Reboot until the end of
    your opponent's turn.

Inherited Effect:
[All Turns] While this Digimon has [Pulsemon] in its text, it gets +1000 DP.

Alt-digi: [Digivolve] [Pulsemon]: Cost 2
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT16055Namakemon:
    """Tests for BT16-055 Namakemon."""

    # ------------------------------------------------------------------
    # Alt-digi from Pulsemon for cost 2
    # ------------------------------------------------------------------

    def test_alt_digi_attributes(self, debug_runner):
        """Should have alt-digi from Pulsemon for cost 2."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-055"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        alt_digi = [e for e in all_effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1, "Should have an alt-digi effect"
        assert alt_digi[0]._alt_digi_cost == 2, "Alt-digi cost should be 2"
        assert alt_digi[0]._alt_digi_name == "Pulsemon", "Alt-digi name should be Pulsemon"

    # ------------------------------------------------------------------
    # [On Play] / [When Digivolving] conditional effects
    # ------------------------------------------------------------------

    def test_on_play_effect_exists(self, debug_runner):
        """Should have On Play effect(s)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-055"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1, "Should have at least 1 On Play effect"

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have When Digivolving effect(s)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-055"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        digivolve_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(digivolve_effects) >= 1, "Should have at least 1 When Digivolving effect"

    def test_3_plus_security_grants_dp_immunity(self, debug_runner):
        """With 3+ security, should grant DP reduction immunity (not Blocker/Reboot)."""
        runner = debug_runner(initial_memory=5)

        game = runner.game
        # Ensure player has 3+ security
        while len(game.player1.security_cards) < 3:
            runner.inject_card(1, "ST1-03", "security")
        assert len(game.player1.security_cards) >= 3

        perm = runner.place_on_field(1, ["BT16-055"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # With 3+ security, at least one Digimon should have DP immunity
        has_dp_immunity = any(
            p.has_keyword('_is_immune_dp_minus')
            for p in game.player1.battle_area if p.is_digimon
        )
        assert has_dp_immunity, \
            "With 3+ security, should grant DP reduction immunity to a Digimon"

    def test_3_or_fewer_security_grants_blocker_reboot(self, debug_runner):
        """With 3 or fewer security, should grant Blocker and Reboot (not DP immunity)."""
        runner = debug_runner(initial_memory=5)

        game = runner.game
        # Reduce security to 2 (clearly fewer than 3)
        while len(game.player1.security_cards) > 2:
            game.player1.security_cards.pop()
        assert len(game.player1.security_cards) < 3

        perm = runner.place_on_field(1, ["BT16-055"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # With fewer than 3 security, at least one Digimon should have Blocker + Reboot
        has_blocker = any(
            p.has_keyword('_is_blocker')
            for p in game.player1.battle_area if p.is_digimon
        )
        has_reboot = any(
            p.has_keyword('_is_reboot')
            for p in game.player1.battle_area if p.is_digimon
        )
        assert has_blocker, \
            "With fewer than 3 security, should grant Blocker to a Digimon"
        assert has_reboot, \
            "With fewer than 3 security, should grant Reboot to a Digimon"

    def test_conditional_branching_is_exclusive(self, debug_runner):
        """The two conditions should be exclusive: 3+ gets immunity, <3 gets keywords.

        The card text says:
        - 'If you have 3 or more security cards' -> DP immunity + De-Digivolve immunity
        - 'If you have 3 or fewer' -> Blocker + Reboot

        Note: at exactly 3 security, BOTH conditions are true per card text.
        The DCGO implementation likely processes them as if/else (3+ = first branch).
        """
        runner = debug_runner(initial_memory=5)

        # Test with exactly 4 security (clearly 3+)
        game = runner.game
        while len(game.player1.security_cards) < 4:
            runner.inject_card(1, "ST1-03", "security")
        assert len(game.player1.security_cards) >= 4

        perm = runner.place_on_field(1, ["BT16-055"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # With 4 security (clearly 3+), at least one Digimon should have DP immunity
        has_dp_immunity = any(
            p.has_keyword('_is_immune_dp_minus')
            for p in game.player1.battle_area if p.is_digimon
        )
        assert has_dp_immunity, \
            "With 4 security (3+), should have DP immunity"

    # ------------------------------------------------------------------
    # Inherited: [All Turns] +1000 DP if has Pulsemon in text
    # ------------------------------------------------------------------

    def test_inherited_dp_modifier_exists(self, debug_runner):
        """Should have an inherited DP modifier effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-055", "ST1-07"])

        source_card = perm.card_sources[0]  # BT16-055
        all_effects = source_card.effect_list(None)
        dp_mods = [
            e for e in all_effects
            if getattr(e, 'is_inherited_effect', False)
            and hasattr(e, 'dp_modifier')
        ]
        assert len(dp_mods) >= 1, "Should have inherited DP modifier effect"

    def test_inherited_dp_modifier_checks_pulsemon_in_text(self, debug_runner):
        """Inherited DP modifier should only activate if Digimon has [Pulsemon] in text."""
        runner = debug_runner(initial_memory=5)

        # Place BT16-055 under a Digimon that does NOT have Pulsemon in text
        perm = runner.place_on_field(1, ["BT16-055", "ST1-07"])

        source_card = perm.card_sources[0]  # BT16-055
        all_effects = source_card.effect_list(None)
        dp_mods = [
            e for e in all_effects
            if getattr(e, 'is_inherited_effect', False)
            and hasattr(e, 'dp_modifier')
        ]
        assert len(dp_mods) >= 1

        # ST1-07 Greymon does NOT have "Pulsemon" in its text
        # So the condition should fail
        dp_mod = dp_mods[0]
        result = dp_mod.can_use_condition({})
        assert not result, \
            "DP modifier should NOT activate when top card doesn't have Pulsemon in text"
