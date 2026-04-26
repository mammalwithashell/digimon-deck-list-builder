"""Behavioral tests for BT20-016 Paildramon (Lv.5 Red/Blue Dragonkin, DP 8000, Cost 8).

Card text:
[On Play] [When Digivolving] For the turn, 1 of your Digimon gains <Piercing>
and gets +4000 DP. Then, this Digimon may attack.
[All Turns] When any of your [Paildramon] or [Dinobeemon] would be deleted,
2 of your Digimon may DNA digivolve into [Imperialdramon: Dragon Mode] in the hand.
Inherited: <Security A. +1>
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT20016Paildramon:
    """Tests for BT20-016 Paildramon."""

    def test_on_play_grants_piercing_to_selected_digimon(self, debug_runner):
        """[On Play] Should grant Piercing to 1 of your Digimon."""
        runner = debug_runner(initial_memory=5)

        # Place Paildramon on field (simulating on-play)
        perm = runner.place_on_field(1, ["BT20-016"])
        # ST1-03 Agumon: Lv.3 Red, DP 2000
        target = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        assert len(on_play_effects) == 1, "Should have exactly 1 On Play effect"

        effect = on_play_effects[0]

        # Mock the selection to auto-select the target
        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            callback(target)
        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Target should have Piercing
        assert target.has_keyword('_is_piercing'), "Target should gain Piercing"
        # Target should have +4000 DP (2000 base + 4000 = 6000)
        assert target.dp == 2000 + 4000, "Target should gain +4000 DP"

    def test_on_play_dp_boost_is_temporary(self, debug_runner):
        """[On Play] DP boost should be cleared at end of turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-016"])
        target = runner.place_on_field(1, ["ST1-03"])  # DP 2000

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            callback(target)
        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert target.dp == 6000, "Should be 2000 + 4000"
        target.clear_temp_dp()
        assert target.dp == 2000, "DP boost should clear at end of turn"

    def test_on_play_piercing_should_be_for_the_turn(self, debug_runner):
        """[On Play] Piercing grant should expire at end of turn (C# uses UntilEachTurnEnd)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-016"])
        target = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            callback(target)
        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert target.has_keyword('_is_piercing'), "Should have Piercing after effect"
        # Simulate end-of-turn keyword expiry (next turn clears grants from this turn)
        target.clear_expired_grants(game.turn_count + 1)
        assert not target.has_keyword('_is_piercing'), \
            "Piercing should expire at end of turn (grant_keyword should use turn-based expiry)"

    def test_on_play_this_digimon_may_attack(self, debug_runner):
        """[On Play] After granting piercing/DP, THIS Digimon (Paildramon) may attack."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-016"])
        target = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            callback(target)
        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # FORCE_ATTACK should be on Paildramon (perm), not on the selected target
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        has_force_attack = game.modifiers.has_modifier(perm, ModifierType.FORCE_ATTACK)
        assert has_force_attack, "FORCE_ATTACK should be on Paildramon itself"

    def test_when_digivolving_same_effect(self, debug_runner):
        """[When Digivolving] Should have the same piercing/DP/attack effect."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-016"])
        target = runner.place_on_field(1, ["ST1-03"])  # DP 2000

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [e for e in effects
                      if e.timing == EffectTiming.OnEnterFieldAnyone
                      and e.is_when_digivolving]
        assert len(wd_effects) == 1, "Should have exactly 1 When Digivolving effect"

        effect = wd_effects[0]

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            callback(target)
        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert target.has_keyword('_is_piercing'), "Target should gain Piercing"
        assert target.dp == 6000, "Target should gain +4000 DP (2000 + 4000)"

    def test_inherited_security_attack_plus_1(self, debug_runner):
        """Inherited effect should provide Security Attack +1."""
        runner = debug_runner(initial_memory=5)

        # Place Paildramon as a digivolution source under another Digimon
        # ST1-11 WarGreymon (Lv.6) on top of BT20-016 Paildramon (Lv.5)
        perm = runner.place_on_field(1, ["BT20-016", "ST1-11"])

        card = perm.card_sources[0]  # Paildramon is the bottom card
        effects = card.effect_list(None)
        sa_effects = [e for e in effects if hasattr(e, '_security_attack_modifier')
                      and e._security_attack_modifier != 0]
        assert len(sa_effects) >= 1, "Should have Security Attack modifier"
        assert sa_effects[0]._security_attack_modifier == 1, "Should be +1"
        assert sa_effects[0].is_inherited_effect, "Should be inherited"
