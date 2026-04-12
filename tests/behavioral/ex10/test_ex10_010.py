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


def _trigger_on_play_effects(runner, perm):
    """Simulate On Play triggers for a placed permanent.

    place_on_field bypasses On Play event processing.  This helper
    fires the on_process_callback of every is_on_play effect so that
    modifier registrations (like CANNOT_BE_AFFECTED) are applied.
    """
    game = runner.game
    card = perm.top_card
    if not card:
        return
    player = card.owner
    for effect in card.effect_list(None):
        if getattr(effect, 'is_on_play', False) and effect.can_use_condition({}):
            cb = getattr(effect, 'on_process_callback', None)
            if cb:
                cb({'player': player, 'game': game, 'permanent': perm})


@pytest.mark.behavioral
class TestEX10010BlackWarGreymon:
    """Tests for EX10-010 BlackWarGreymon."""

    # ── Keywords ─────────────────────────────────────────────────────

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

    # ── On Play: Delete opponent's Digimon/Tamer with play cost <= 7 ──

    def test_on_play_delete_play_cost_7_or_less(self, debug_runner):
        """[On Play] Should delete 1 opponent's Digimon/Tamer with play cost 7 or less."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # ST1-03 Agumon: Lv.3, play_cost 3 (should be deletable)
        target = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_delete_effects = [e for e in effects
                                  if e.timing == EffectTiming.OnEnterFieldAnyone
                                  and e.is_on_play
                                  and "Delete" in (e.effect_description or "")]
        assert len(on_play_delete_effects) >= 1, "Should have On Play delete effect"

        effect = on_play_delete_effects[0]
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
        on_play_delete_effects = [e for e in effects
                                  if e.timing == EffectTiming.OnEnterFieldAnyone
                                  and e.is_on_play
                                  and "Delete" in (e.effect_description or "")]
        effect = on_play_delete_effects[0]

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

    def test_on_play_condition_requires_field_presence(self, debug_runner):
        """[On Play] condition should check permanent_of_this_card() is not None."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        # While on field, condition should be True
        assert effect.can_use_condition({}) is True, \
            "On Play condition should be True when card is on field"

        # Remove from field — condition should be False
        game.player1.battle_area.remove(perm)
        assert effect.can_use_condition({}) is False, \
            "On Play condition should be False when card leaves field"

    # ── When Digivolving: Same delete effect ────────────────────────

    def test_when_digivolving_delete_effect(self, debug_runner):
        """[When Digivolving] Should also have delete effect for Digimon/Tamer cost <= 7."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])

        card = perm.top_card
        effects = card.effect_list(None)
        wd_delete_effects = [e for e in effects
                             if e.timing == EffectTiming.OnEnterFieldAnyone
                             and e.is_when_digivolving
                             and "Delete" in (e.effect_description or "")]
        assert len(wd_delete_effects) >= 1, "Should have When Digivolving delete effect"

    # ── All Turns: DP +3000 when opponent has 13000+ DP ─────────────

    def test_dp_boost_when_opponent_has_13k(self, debug_runner):
        """[All Turns] Should get +3000 DP when opponent has a 13000+ DP Digimon.

        EX10-010 base DP = 12000. With boost = 15000.
        """
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # BT20-102 Omnimon X: DP 16000 (>= 13000, triggers the condition)
        runner.place_on_field(2, ["BT20-102"])

        # Check actual DP on the permanent (12000 base + 3000 boost = 15000)
        assert perm.dp == 15000, \
            f"DP should be 15000 (12000 base + 3000 boost), got {perm.dp}"

    def test_no_dp_boost_without_13k_opponent(self, debug_runner):
        """[All Turns] Should NOT get +3000 DP when opponent has no 13000+ DP Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # ST1-03 Agumon: DP 2000 (way under 13000)
        runner.place_on_field(2, ["ST1-03"])

        # Base DP should be unchanged
        assert perm.dp == 12000, \
            f"DP should be 12000 (no boost), got {perm.dp}"

    def test_dp_boost_condition_checks_effect_level(self, debug_runner):
        """[All Turns] DP boost condition should use effect's can_use_condition."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # BT20-102 Omnimon X: DP 16000 (>= 13000)
        runner.place_on_field(2, ["BT20-102"])

        card = perm.top_card
        effects = card.effect_list(None)
        dp_effects = [e for e in effects if getattr(e, 'dp_modifier', 0) == 3000]
        assert len(dp_effects) >= 1, "Should have +3000 DP conditional effect"

        dp_effect = dp_effects[0]
        # Condition should pass (opponent has 16000 DP Digimon)
        assert dp_effect.can_use_condition({}), \
            "DP condition should pass when opponent has 13000+ DP Digimon"

    def test_dp_boost_threshold_exactly_13000(self, debug_runner):
        """[All Turns] Exactly 13000 DP should also trigger the boost (>= 13000)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # BT2-083 Diaboromon: DP 13000 exactly
        opp = runner.place_on_field(2, ["BT2-083"])

        assert opp.dp == 13000, f"BT2-083 should have 13000 DP, got {opp.dp}"
        assert perm.dp == 15000, \
            f"DP should be 15000 when opp has exactly 13000 DP, got {perm.dp}"

    # ── All Turns: Effect immunity when opponent has 13000+ DP ───────

    def test_immunity_prevents_targeting_by_opp_effects(self, debug_runner):
        """[All Turns] BlackWarGreymon should be immune to opponent's effects
        when opponent has a 13000+ DP Digimon.

        The immunity should use CANNOT_BE_AFFECTED modifier so that
        effect_select_opponent_permanent skips this permanent.
        """
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # BT20-102: DP 16000 (triggers the condition)
        runner.place_on_field(2, ["BT20-102"])

        # Trigger On Play to register the CANNOT_BE_AFFECTED modifier
        _trigger_on_play_effects(runner, perm)

        game = runner.game

        # The engine's effect_select_opponent_permanent checks
        # modifiers.can_be_selected_by_effect(). With CANNOT_BE_AFFECTED
        # registered, BlackWarGreymon should NOT be selectable.
        can_select = game.modifiers.can_be_selected_by_effect(perm)
        assert not can_select, \
            "BlackWarGreymon should NOT be selectable by opponent effects " \
            "when opponent has 13000+ DP Digimon (CANNOT_BE_AFFECTED modifier)"

    def test_no_immunity_without_13k_opponent(self, debug_runner):
        """[All Turns] BlackWarGreymon should NOT have immunity when opponent
        has no 13000+ DP Digimon.
        """
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])
        # ST1-03 Agumon: DP 2000 (no trigger)
        runner.place_on_field(2, ["ST1-03"])

        # Trigger On Play to register the CANNOT_BE_AFFECTED modifier
        _trigger_on_play_effects(runner, perm)

        game = runner.game

        # The modifier condition checks _opp_has_13k_dp() dynamically.
        # With only a 2000 DP opponent, the condition should be False.
        can_select = game.modifiers.can_be_selected_by_effect(perm)
        assert can_select, \
            "BlackWarGreymon SHOULD be selectable when opponent has no 13000+ DP Digimon"

    def test_immunity_dynamic_with_opp_field_changes(self, debug_runner):
        """[All Turns] Immunity should activate/deactivate dynamically
        as opponent's field changes.
        """
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-010"])

        # Trigger On Play to register the CANNOT_BE_AFFECTED modifier
        _trigger_on_play_effects(runner, perm)

        game = runner.game

        # Initially no opponent Digimon -- should be selectable
        assert game.modifiers.can_be_selected_by_effect(perm), \
            "Should be selectable with no opponent Digimon"

        # Place a big Digimon
        big_opp = runner.place_on_field(2, ["BT20-102"])  # 16000 DP
        assert not game.modifiers.can_be_selected_by_effect(perm), \
            "Should NOT be selectable after placing 13000+ DP opponent"

        # Remove the big Digimon -- immunity should deactivate
        game.player2.battle_area.remove(big_opp)
        assert game.modifiers.can_be_selected_by_effect(perm), \
            "Should be selectable again after removing the 13000+ DP opponent"

    # ── Ace Overflow inherited ───────────────────────────────────────

    def test_ace_overflow_inherited_in_metadata(self, debug_runner):
        """Inherited: Ace Overflow <-4> should be parsed from card metadata.

        The engine handles Ace Overflow via entity_base.is_ace and
        entity_base.ace_overflow_cost — no script code needed.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-010"])

        card = perm.top_card
        assert card.c_entity_base.is_ace, \
            "EX10-010 should be recognized as an ACE card"
        assert card.c_entity_base.ace_overflow_cost == 4, \
            f"Ace Overflow cost should be 4, got {card.c_entity_base.ace_overflow_cost}"
