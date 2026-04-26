"""Behavioral tests for BT21-015 Cyclonemon (Lv.4 Red Digimon, 5000 DP).

Card text:
[Security] At the end of the battle, play this card without paying the cost.
[On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 4000 DP or less.

Inherited Effect:
[Your Turn] This Digimon gets +2000 DP.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT21015Cyclonemon:
    """Tests for BT21-015 Cyclonemon."""

    # ── Security Effect ──────────────────────────────────────────────

    def test_security_effect_flag_set(self, debug_runner):
        """The script must have is_security_effect = True on the security play effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT21-015"])

        card = perm.top_card
        effects = card.effect_list(None)
        sec_effects = [e for e in effects if e.is_security_effect]
        assert len(sec_effects) == 1, (
            f"Cyclonemon should have exactly 1 security effect, found {len(sec_effects)}"
        )

    # ── On Play: Delete opponent Digimon with 4000 DP or less ────────

    def test_on_play_delete_opponent_digimon_at_4000dp(self, debug_runner):
        """On Play should trigger delete selection for opponent Digimon with DP <= 4000."""
        runner = debug_runner(initial_memory=8)

        # Place a 4000 DP target on opponent's field
        target = runner.place_on_field(2, ["ST1-03"])  # Agumon, base 3000 DP
        # Manually set DP to exactly 4000 for precision
        target.card_sources[0].c_entity_base.dp = 4000

        # Place Cyclonemon on P1's field to trigger On Play
        cyclonemon_perm = runner.place_on_field(1, ["BT21-015"])

        game = runner.game
        card = cyclonemon_perm.top_card
        effects = card.effect_list(None)

        # Find the on_play effect
        on_play_effects = [e for e in effects if e.is_on_play and
                           e.timing == EffectTiming.OnEnterFieldAnyone]
        assert len(on_play_effects) >= 1, "Should have at least 1 On Play effect"

        on_play = on_play_effects[0]
        assert on_play.can_use_condition({}), "On Play condition should pass when on field"

        # Execute the effect
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': cyclonemon_perm,
        })

        # The game should enter a selection phase to pick the target
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Game should be in SelectTarget phase, got {game.current_phase}"
        )

    def test_on_play_does_not_target_above_4000dp(self, debug_runner):
        """On Play should NOT target Digimon with DP > 4000."""
        runner = debug_runner(initial_memory=8)

        # Place a 5000 DP Digimon on opponent's field
        big_target = runner.place_on_field(2, ["BT21-015"])  # Cyclonemon itself, 5000 DP

        cyclonemon = runner.place_on_field(1, ["BT21-015"])

        game = runner.game
        card = cyclonemon.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play and
                   e.timing == EffectTiming.OnEnterFieldAnyone][0]

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': cyclonemon,
        })

        # No valid targets, so game should NOT enter selection phase
        assert game.current_phase != GamePhase.SelectTarget, (
            "No Digimon with 4000 DP or less exists; should not enter selection"
        )

    def test_on_play_deletes_selected_target(self, debug_runner):
        """On Play delete should remove the selected opponent Digimon to trash."""
        runner = debug_runner(initial_memory=8)

        # Place a weak Digimon on opponent's field
        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        weak_target = make_permanent(card_id="TEST-001", name="WeakMon", dp=3000, level=3)
        game = runner.game
        game.player2.battle_area.append(weak_target)

        cyclonemon = runner.place_on_field(1, ["BT21-015"])

        card = cyclonemon.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play and
                   e.timing == EffectTiming.OnEnterFieldAnyone][0]

        p2_field_before = len(game.player2.battle_area)

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': cyclonemon,
        })

        # Auto-resolve the selection
        results = runner.auto_resolve()

        # After selection + deletion, opponent should have 1 fewer Digimon
        assert len(game.player2.battle_area) < p2_field_before, (
            "Opponent should have lost a Digimon after delete resolved"
        )

    # ── When Digivolving: Same delete effect ─────────────────────────

    def test_when_digivolving_delete_effect_exists(self, debug_runner):
        """When Digivolving should also have a delete effect for opponent Digimon <= 4000 DP."""
        runner = debug_runner(initial_memory=8)
        perm = runner.place_on_field(1, ["BT21-015"])

        card = perm.top_card
        effects = card.effect_list(None)

        digi_effects = [e for e in effects if e.is_when_digivolving and
                        e.timing == EffectTiming.OnEnterFieldAnyone]
        assert len(digi_effects) >= 1, (
            "Cyclonemon should have a When Digivolving delete effect"
        )

    def test_when_digivolving_delete_targets_4000_or_less(self, debug_runner):
        """When Digivolving delete filter should match Digimon with DP <= 4000."""
        runner = debug_runner(initial_memory=8)

        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        weak = make_permanent(card_id="TEST-002", name="WeakMon", dp=4000, level=3)
        game = runner.game
        game.player2.battle_area.append(weak)

        cyclonemon = runner.place_on_field(1, ["BT21-015"])

        card = cyclonemon.top_card
        effects = card.effect_list(None)
        digi_eff = [e for e in effects if e.is_when_digivolving and
                    e.timing == EffectTiming.OnEnterFieldAnyone][0]

        digi_eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': cyclonemon,
        })

        # Should enter selection phase (4000 DP target qualifies)
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should enter SelectTarget with 4000 DP target, got {game.current_phase}"
        )

    # ── Inherited Effect: [Your Turn] +2000 DP ──────────────────────

    def test_inherited_dp_modifier_value(self, debug_runner):
        """Inherited effect should give +2000 DP."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT21-015"])

        card = perm.top_card
        effects = card.effect_list(None)

        inherited_dp = [e for e in effects if e.is_inherited_effect and e.dp_modifier]
        assert len(inherited_dp) == 1, "Should have exactly 1 inherited DP modifier effect"
        assert inherited_dp[0].dp_modifier == 2000, (
            f"Inherited DP modifier should be 2000, got {inherited_dp[0].dp_modifier}"
        )

    def test_inherited_dp_only_your_turn(self, debug_runner):
        """Inherited DP +2000 should only be active on the owner's turn."""
        runner = debug_runner(initial_memory=5)

        # Place Cyclonemon as a digivolution source under another Digimon
        # Use ST1-07 Greymon as the Lv.5 on top
        perm = runner.place_on_field(1, ["BT21-015", "ST1-07"])

        card = perm.card_sources[0]  # Cyclonemon (bottom)
        effects = card.effect_list(None)
        inherited_dp = [e for e in effects if e.is_inherited_effect and e.dp_modifier][0]

        # On player1's turn, condition should pass
        game = runner.game
        game.player1.is_my_turn = True
        assert inherited_dp.can_use_condition({}), (
            "Inherited DP should be active on owner's turn"
        )

        # On opponent's turn, condition should fail
        game.player1.is_my_turn = False
        assert not inherited_dp.can_use_condition({}), (
            "Inherited DP should NOT be active on opponent's turn"
        )

    def test_inherited_dp_applies_to_stack(self, debug_runner):
        """When Cyclonemon is in a digivolution stack, the parent should get +2000 DP
        on the owner's turn."""
        runner = debug_runner(initial_memory=5)

        # ST1-07 Greymon is a Lv.5 red Digimon with base 6000 DP
        perm = runner.place_on_field(1, ["BT21-015", "ST1-07"])

        game = runner.game
        game.player1.is_my_turn = True

        # The permanent's DP should include the +2000 inherited modifier
        base_dp = perm.top_card.c_entity_base.dp  # ST1-07 base DP
        actual_dp = perm.dp
        assert actual_dp == base_dp + 2000, (
            f"Stack DP should be {base_dp} + 2000 = {base_dp + 2000}, got {actual_dp}"
        )

    def test_inherited_dp_not_on_opponent_turn(self, debug_runner):
        """Inherited +2000 DP should NOT apply on opponent's turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT21-015", "ST1-07"])

        game = runner.game
        game.player1.is_my_turn = False

        base_dp = perm.top_card.c_entity_base.dp
        actual_dp = perm.dp
        assert actual_dp == base_dp, (
            f"Stack DP on opponent's turn should be base {base_dp}, got {actual_dp}"
        )

    # ── On Play: Mandatory (not optional) ────────────────────────────

    def test_on_play_delete_is_mandatory(self, debug_runner):
        """The delete effect should be mandatory (is_optional=False in the select call)."""
        runner = debug_runner(initial_memory=8)
        perm = runner.place_on_field(1, ["BT21-015"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play and
                   e.timing == EffectTiming.OnEnterFieldAnyone][0]

        # The effect should not be marked as optional
        assert not on_play.is_optional, "On Play delete should be mandatory"
