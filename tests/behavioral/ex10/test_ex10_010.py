"""Behavioral tests for EX10-010 BlackWarGreymon (Lv.6 Red/Black Dragonkin, DP 12000, Cost 7).

Card text:
[Hand] [Counter] <Blast Digivolve>
<Raid> <Reboot> <Blocker>
[On Play] [When Digivolving] Delete 1 of your opponent's play cost 7 or lower
Digimon or Tamers.
[All Turns] While your opponent has a Digimon with 13000 DP or more, your
opponent's Digimon's effects don't affect this Digimon, and it gets +3000 DP.
Inherited: Ace Overflow <-4>
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX10010BlackWarGreymon:
    """Tests for EX10-010 BlackWarGreymon."""

    def test_has_blast_digivolve(self, debug_runner):
        """Should have Blast Digivolve keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-010"])

        card = perm.top_card
        effects = card.effect_list(None)
        blast_effects = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast_effects) >= 1, "Should have Blast Digivolve"

    def test_has_raid(self, debug_runner):
        """Should have Raid keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-010"])

        card = perm.top_card
        effects = card.effect_list(None)
        raid_effects = [e for e in effects if getattr(e, '_is_raid', False)]
        assert len(raid_effects) >= 1, "Should have Raid"

    def test_has_reboot(self, debug_runner):
        """Should have Reboot keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-010"])

        card = perm.top_card
        effects = card.effect_list(None)
        reboot_effects = [e for e in effects if getattr(e, '_is_reboot', False)]
        assert len(reboot_effects) >= 1, "Should have Reboot"

    def test_has_blocker(self, debug_runner):
        """Should have Blocker keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-010"])

        card = perm.top_card
        effects = card.effect_list(None)
        blocker_effects = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker_effects) >= 1, "Should have Blocker"

    def test_on_play_delete_play_cost_7_or_less(self, debug_runner):
        """[On Play] Should delete 1 opponent's Digimon/Tamer with play cost 7 or less."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # ST1-03 Agumon: Lv.3, play_cost 3 (should be deletable)
        target = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        assert len(on_play_effects) == 1, "Should have On Play delete effect"

        effect = on_play_effects[0]
        deleted_perms = []

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_opp

        original_delete = game.player2.delete_permanent
        def track_delete(p):
            deleted_perms.append(p)
            original_delete(p)
        game.player2.delete_permanent = track_delete

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert target in deleted_perms, "Should delete opponent's Digimon with cost <= 7"

    def test_on_play_filter_excludes_high_cost(self, debug_runner):
        """[On Play] Should NOT target opponent's Digimon with play cost > 7."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # ST1-11 WarGreymon: play_cost 12 (too high)
        target_high = runner.place_on_field(2, ["ST1-11"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        selected = []
        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn and filter_fn(p):
                    selected.append(p)
        game.effect_select_opponent_permanent = mock_select_opp

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert target_high not in selected, \
            "High play cost Digimon should NOT be a valid target"

    def test_dp_boost_when_opponent_has_13k(self, debug_runner):
        """[All Turns] Should get +3000 DP when opponent has a 13000+ DP Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # ST1-11 WarGreymon: DP 12000 (NOT enough)
        # BT20-102 Omnimon X: DP 16000 (enough!)
        opp = runner.place_on_field(2, ["BT20-102"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        dp_effects = [e for e in effects if getattr(e, 'dp_modifier', 0) == 3000]
        assert len(dp_effects) >= 1, "Should have +3000 DP conditional effect"

        dp_effect = dp_effects[0]
        # Condition should pass (opponent has 16000 DP Digimon)
        assert dp_effect.can_use_condition({}), \
            "DP condition should pass when opponent has 13000+ DP Digimon"

    def test_dp_boost_uses_permanent_dp_property(self, debug_runner):
        """[All Turns] The 13000 DP check should use permanent.dp (not current_dp).

        Permanent has a `dp` property. There is no `current_dp` property.
        Using current_dp would raise AttributeError.
        """
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # BT20-102 has DP 16000, which is >= 13000
        opp = runner.place_on_field(2, ["BT20-102"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        dp_effects = [e for e in effects if getattr(e, 'dp_modifier', 0) == 3000]
        dp_effect = dp_effects[0]

        # This should NOT raise AttributeError
        result = dp_effect.can_use_condition({})
        assert result, "Should pass when opponent has 13000+ DP Digimon"

    def test_no_dp_boost_without_13k_opponent(self, debug_runner):
        """[All Turns] Should NOT get +3000 DP when opponent has no 13000+ DP Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # ST1-03 Agumon: DP 2000 (way under 13000)
        opp = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        dp_effects = [e for e in effects if getattr(e, 'dp_modifier', 0) == 3000]
        dp_effect = dp_effects[0]

        assert not dp_effect.can_use_condition({}), \
            "DP condition should fail when no opponent Digimon has 13000+ DP"

    def test_immunity_when_opponent_has_13k(self, debug_runner):
        """[All Turns] Should have effect immunity when opponent has 13000+ DP Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        opp = runner.place_on_field(2, ["BT20-102"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        immunity_effects = [e for e in effects
                            if getattr(e, '_is_immune_to_opponent_digimon_effects', False)]
        assert len(immunity_effects) >= 1, "Should have immunity effect"

        imm_effect = immunity_effects[0]
        assert imm_effect.can_use_condition({}), \
            "Immunity condition should pass when opponent has 13000+ DP Digimon"
