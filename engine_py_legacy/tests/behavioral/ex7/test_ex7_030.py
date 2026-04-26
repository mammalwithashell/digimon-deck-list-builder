"""Behavioral tests for EX7-030 Cendrillmon (Lv.6, Yellow, Puppet/LIBERATOR).

Card text:
<Overclock ([Puppet] Trait)>
[Start of Your Main Phase] [When Digivolving] You may play 1 [Familiar] Token.
    (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets
    -3000 DP for the turn.)
[When Attacking] 1 of your opponent's Digimon gets -6000 DP for the turn.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX7030Cendrillmon:
    """Tests for EX7-030 Cendrillmon effects."""

    # ── Helper ────────────────────────────────────────────────────────

    def _get_effects(self, perm):
        """Return all effects from the top card (Cendrillmon itself)."""
        top_card = perm.top_card
        return top_card.effect_list(None)

    def _find_effects_by_timing(self, perm, timing):
        effects = self._get_effects(perm)
        return [e for e in effects if e.timing == timing]

    # ── Effect structure tests ────────────────────────────────────────

    def test_has_overclock_effect(self, debug_runner):
        """Effect 0 should be an Overclock effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        effects = self._get_effects(perm)
        overclock_effects = [e for e in effects if getattr(e, '_is_overclock', False)]
        assert len(overclock_effects) == 1, "Should have exactly 1 Overclock effect"

    def test_has_start_main_phase_token_effect(self, debug_runner):
        """Should have a [Start of Your Main Phase] effect that is optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        smp_effects = self._find_effects_by_timing(perm, EffectTiming.OnStartMainPhase)
        assert len(smp_effects) == 1, "Should have 1 OnStartMainPhase effect"
        assert smp_effects[0].is_optional, "Start of Main Phase token effect should be optional"

    def test_has_when_digivolving_token_effect(self, debug_runner):
        """Should have a [When Digivolving] effect that is optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        effects = self._get_effects(perm)
        digi_effects = [e for e in effects if getattr(e, 'is_when_digivolving', False)]
        assert len(digi_effects) == 1, "Should have 1 When Digivolving effect"
        assert digi_effects[0].is_optional, "When Digivolving token effect should be optional"

    def test_has_when_attacking_dp_effect(self, debug_runner):
        """Should have a [When Attacking] effect using OnUseAttack timing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        atk_effects = self._find_effects_by_timing(perm, EffectTiming.OnUseAttack)
        assert len(atk_effects) == 1, "Should have 1 OnUseAttack effect"
        assert atk_effects[0].is_on_attack, "Effect should have is_on_attack flag"

    def test_when_attacking_is_not_optional(self, debug_runner):
        """[When Attacking] -6000 DP effect must NOT be optional (C# isOptional: false)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        atk_effects = self._find_effects_by_timing(perm, EffectTiming.OnUseAttack)
        assert len(atk_effects) == 1
        assert not atk_effects[0].is_optional, "[When Attacking] effect must be mandatory"

    # ── Behavioral: Start of Main Phase token ─────────────────────────

    def test_start_main_phase_plays_familiar_token(self, debug_runner):
        """[Start of Main Phase] should play a Familiar Token on player's field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        game = runner.game

        initial_field_count = len(game.player1.battle_area)

        smp_effects = self._find_effects_by_timing(perm, EffectTiming.OnStartMainPhase)
        assert len(smp_effects) == 1
        effect = smp_effects[0]

        # Condition should pass (on field, owner's turn)
        assert effect.can_use_condition({}), "Condition should pass on owner's turn"

        # Execute
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have one more permanent (the Familiar Token)
        assert len(game.player1.battle_area) == initial_field_count + 1
        token_perm = game.player1.battle_area[-1]
        assert token_perm.top_card.is_token, "New permanent should be a token"
        assert 'Familiar' in token_perm.top_card.c_entity_base.card_name_eng, \
            "Token should be a Familiar Token"

    # ── Behavioral: When Digivolving token ────────────────────────────

    def test_when_digivolving_plays_familiar_token(self, debug_runner):
        """[When Digivolving] should play a Familiar Token on player's field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        game = runner.game

        initial_field_count = len(game.player1.battle_area)

        effects = self._get_effects(perm)
        digi_effects = [e for e in effects if getattr(e, 'is_when_digivolving', False)]
        assert len(digi_effects) == 1
        effect = digi_effects[0]

        # Execute
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.battle_area) == initial_field_count + 1
        token_perm = game.player1.battle_area[-1]
        assert token_perm.top_card.is_token

    # ── Behavioral: When Attacking -6000 DP ───────────────────────────

    def test_when_attacking_reduces_opponent_dp(self, debug_runner):
        """[When Attacking] should reduce 1 opponent Digimon's DP by 6000."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        game = runner.game

        # Place an opponent Digimon with DP > 6000 so reduction is visible
        # EX7-030 Cendrillmon itself: Lv.6, 12000 DP
        opp_perm = runner.place_on_field(2, ["EX7-030"])
        opp_base_dp = opp_perm.dp
        assert opp_base_dp >= 6000, f"Need base DP >= 6000 for this test, got {opp_base_dp}"

        atk_effects = self._find_effects_by_timing(perm, EffectTiming.OnUseAttack)
        assert len(atk_effects) == 1
        effect = atk_effects[0]

        # Condition should pass
        assert effect.can_use_condition({}), "Condition should pass when on field"

        # Execute — queues a SelectTarget phase
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Auto-resolve the pending target selection
        runner.auto_resolve()

        # Opponent's Digimon should have -6000 DP modifier
        assert opp_perm.dp == opp_base_dp - 6000, \
            f"Opponent DP should be {opp_base_dp - 6000} but got {opp_perm.dp}"

    def test_when_attacking_no_targets_noop(self, debug_runner):
        """[When Attacking] should be a no-op if opponent has no Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        game = runner.game

        # Ensure opponent has no Digimon on field (default state after setup)
        assert len(game.player2.battle_area) == 0, "Opponent should have empty field"

        atk_effects = self._find_effects_by_timing(perm, EffectTiming.OnUseAttack)
        effect = atk_effects[0]

        # Should not crash when there are no valid targets
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

    # ── Timing filter: OnUseAttack only fires for self ────────────────

    def test_on_use_attack_only_fires_for_self(self, debug_runner):
        """OnUseAttack should only trigger when THIS perm is the attacker,
        not when another ally attacks."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX7-030"])
        game = runner.game

        atk_effects = self._find_effects_by_timing(perm, EffectTiming.OnUseAttack)
        effect = atk_effects[0]

        # Place another permanent to use as a different attacker
        other_perm = runner.place_on_field(1, ["ST1-07"])

        # OnUseAttack should match when perm IS the attacker
        result_self = game._effect_matches_timing(
            effect, EffectTiming.OnUseAttack, perm,
            played_card=None, digivolved_perm=None,
            extra_context={'attacker': perm},
        )
        assert result_self, "OnUseAttack should trigger when perm is the attacker"

        # OnUseAttack should NOT match when another perm is the attacker
        result_other = game._effect_matches_timing(
            effect, EffectTiming.OnUseAttack, perm,
            played_card=None, digivolved_perm=None,
            extra_context={'attacker': other_perm},
        )
        assert not result_other, "OnUseAttack should NOT trigger for a different attacker"
