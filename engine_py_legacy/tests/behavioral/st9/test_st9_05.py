"""Behavioral tests for ST9-05 Paildramon (Lv.5 Blue/Green Digimon).

Card text:
[When Digivolving] When DNA digivolving, return 1 of your opponent's Digimon
with 6000 DP or less to the bottom of its owner's deck.
[When Attacking] [Once Per Turn] Unsuspend this Digimon.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST9_05Paildramon:
    """Tests for ST9-05 Paildramon."""

    # ---------------------------------------------------------------
    # Clause 1: [When Digivolving] DNA-only bounce <=6000 DP
    # ---------------------------------------------------------------

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Paildramon should have a When Digivolving effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd) == 1, "Should have exactly 1 When Digivolving effect"

    def test_when_digivolving_requires_dna(self, debug_runner):
        """WD condition should require DNA digivolve context."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        # Without is_dna_digivolve: should fail
        result = wd.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is False, "WD condition should fail without DNA digivolve"

        # With is_dna_digivolve=False: should fail
        result = wd.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })
        assert result is False, "WD condition should fail with is_dna_digivolve=False"

        # With is_dna_digivolve=True: should pass
        result = wd.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': True,
        })
        assert result is True, "WD condition should pass with is_dna_digivolve=True"

    def test_when_digivolving_returns_low_dp_digimon(self, debug_runner):
        """DNA digivolve should return opponent Digimon with <=6000 DP to deck bottom."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        # Place opponent Digimon: ST1-03 Agumon (2000 DP) - valid target
        opp = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        opp_field_before = len(game.player2.battle_area)
        opp_lib_before = len(game.player2.library_cards)

        # Execute with DNA context
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
            'is_dna_digivolve': True,
        })

        # Should be in target selection
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", "Should enter target selection"

        legal = runner.action_mask()
        target_actions = [a for a in legal if a != 62]
        assert target_actions, "Should have valid targets"
        runner.execute(target_actions[0])
        runner.auto_resolve()

        # Agumon should be gone from field
        snap = runner.snapshot()
        opp_names = [s.card_name for s in snap.p2_field]
        assert "Agumon" not in opp_names, "Agumon should be returned to deck"

        # Library should have gained the card
        assert len(game.player2.library_cards) > opp_lib_before, (
            "Opponent's library should have gained the returned card"
        )

    def test_when_digivolving_rejects_high_dp_digimon(self, debug_runner):
        """Should not be able to target opponent Digimon with >6000 DP."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        # Place opponent Digimon with >6000 DP
        # ST1-07 Greymon has 4000 DP - but we need a high-DP target
        # ST1-11 WarGreymon has 12000 DP
        runner.place_on_field(2, ["ST1-07", "ST1-11"])  # Stack: Greymon under WarGreymon

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        # Execute with DNA context
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
            'is_dna_digivolve': True,
        })

        # With no valid targets (>6000 DP), should NOT enter selection phase
        snap = runner.snapshot()
        assert snap.phase != "SelectTarget", (
            "Should not enter target selection when no valid targets (all >6000 DP)"
        )

    def test_when_digivolving_filters_dp_correctly(self, debug_runner):
        """With mixed DP targets, only <=6000 DP should be selectable."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        # Place two opponent Digimon: one <=6000, one >6000
        low_dp = runner.place_on_field(2, ["ST1-03"])   # Agumon, 2000 DP
        high_dp = runner.place_on_field(2, ["ST1-07", "ST1-11"])  # WarGreymon, 12000 DP

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
            'is_dna_digivolve': True,
        })

        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", "Should enter selection (Agumon is valid)"

        legal = runner.action_mask()
        target_actions = [a for a in legal if a != 62]
        # Should only have 1 valid target (Agumon, not WarGreymon)
        assert len(target_actions) == 1, (
            f"Should have exactly 1 valid target (low DP only), got {len(target_actions)}"
        )

    def test_when_digivolving_exactly_6000_dp_is_valid(self, debug_runner):
        """A Digimon with exactly 6000 DP should be a valid target (<=6000)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        # BT21-037 Lighdramon is 6000 DP
        runner.place_on_field(2, ["BT21-037"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
            'is_dna_digivolve': True,
        })

        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", "6000 DP Digimon should be valid target"

    def test_when_digivolving_requires_field_presence(self, debug_runner):
        """WD condition should require the card to be on field."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        # On field with DNA: pass
        assert wd.can_use_condition({
            'player': game.player1, 'game': game, 'permanent': perm,
            'is_dna_digivolve': True,
        })

        # Off field
        game.player1.battle_area.remove(perm)
        result = wd.can_use_condition({
            'player': game.player1, 'game': game,
            'is_dna_digivolve': True,
        })
        assert result is False, "WD condition should fail when not on field"

    # ---------------------------------------------------------------
    # Clause 2: [When Attacking] [Once Per Turn] Unsuspend self
    # ---------------------------------------------------------------

    def test_when_attacking_effect_exists(self, debug_runner):
        """Paildramon should have a When Attacking OPT unsuspend effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        top = perm.top_card
        effects = top.effect_list(None)
        oa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ]
        assert len(oa) == 1, "Should have exactly 1 OnUseAttack effect"

    def test_when_attacking_is_optional(self, debug_runner):
        """The unsuspend effect should be optional."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        top = perm.top_card
        effects = top.effect_list(None)
        oa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ][0]
        assert oa.is_optional is True, "Unsuspend effect should be optional"

    def test_when_attacking_once_per_turn(self, debug_runner):
        """The unsuspend effect should be Once Per Turn."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        top = perm.top_card
        effects = top.effect_list(None)
        oa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ][0]

        assert oa.max_count_per_turn == 1, "Should be once per turn"
        assert oa.can_activate_this_turn(), "First use should be allowed"
        oa.record_activation()
        assert not oa.can_activate_this_turn(), "Second use should be blocked"

    def test_when_attacking_unsuspends_self(self, debug_runner):
        """Process should unsuspend this Digimon."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])
        perm.suspend()
        assert perm.is_suspended, "Paildramon should be suspended"

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        oa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ][0]

        oa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert not perm.is_suspended, "Paildramon should be unsuspended after effect"

    def test_when_attacking_no_crash_when_already_unsuspended(self, debug_runner):
        """Process should not crash if Digimon is already unsuspended."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])
        # Not suspended
        assert not perm.is_suspended

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        oa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ][0]

        # Should not crash
        oa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert not perm.is_suspended, "Should remain unsuspended"

    def test_when_attacking_requires_field_presence(self, debug_runner):
        """Unsuspend condition should require field presence."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["ST9-03", "ST9-05"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        oa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ][0]

        # On field: should pass
        assert oa.can_use_condition({
            'player': game.player1, 'game': game, 'permanent': perm,
        })

        # Off field: should fail
        game.player1.battle_area.remove(perm)
        result = oa.can_use_condition({
            'player': game.player1, 'game': game,
        })
        assert result is False, "Condition should fail when not on field"
