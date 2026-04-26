"""Behavioral tests for BT22-056 Guardromon.

Card text:
    [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP
        for the turn. Then, if this Digimon's stack has 2 or more same-level
        cards, <De-Digivolve 1> 1 of your opponent's Digimon.
        (Trash the top card. You can't trash past level 3 cards.)

Inherited effect:
    [Opponent's Turn] This Digimon gets +2000 DP.

Alt-digi: Digivolve from Lv.3 [CS] Digimon for cost 2.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


def _get_effects(perm):
    """Get all effects from all card sources on a permanent."""
    effects = []
    for source in perm.card_sources:
        effects.extend(source.effect_list(EffectTiming.NoTiming))
    return effects


def _find_on_play_effect(perm):
    for eff in _get_effects(perm):
        if getattr(eff, 'is_on_play', False):
            return eff
    return None


def _find_wd_effect(perm):
    for eff in _get_effects(perm):
        if getattr(eff, 'is_when_digivolving', False):
            return eff
    return None


def _find_inherited_dp_effect(perm):
    """Find the inherited +2000 DP effect on the Guardromon card source."""
    # Scan the actual BT22-056 card source (not the permanent's active effects)
    for source in perm.card_sources:
        if source.c_entity_base and source.c_entity_base.card_id == "BT22-056":
            for eff in source.effect_list(EffectTiming.NoTiming):
                if (getattr(eff, 'is_inherited_effect', False)
                        and eff.dp_modifier == 2000):
                    return eff
    return None


@pytest.mark.behavioral
class TestBT22056Guardromon:
    """Tests for BT22-056 Guardromon."""

    # ── C1/C2: [On Play] / [When Digivolving] -3000 DP ───────────────

    def test_on_play_applies_minus_3000_to_selected_opponent(self, debug_runner):
        """On Play selects 1 opponent Digimon and applies -3000 DP (end of turn)."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-06"] * 50,
            initial_memory=10,
        )
        guardromon = runner.place_on_field(1, ["BT22-056"])
        opp = runner.place_on_field(2, ["ST1-06"])  # Coredramon 6000 DP
        assert opp.dp == 6000

        game = runner.game
        op_effect = _find_on_play_effect(guardromon)
        assert op_effect is not None, "Should find On Play effect"

        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        op_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        assert opp.dp == 3000, (
            f"Opponent should have 3000 DP (6000 - 3000). Got {opp.dp}"
        )

    def test_on_play_dp_debuff_does_not_leak_to_other_opponents(self, debug_runner):
        """CRITICAL: -3000 DP must only apply to the selected target, not all opponents."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-06"] * 50,
            initial_memory=10,
        )
        guardromon = runner.place_on_field(1, ["BT22-056"])
        opp1 = runner.place_on_field(2, ["ST1-06"])  # 6000 DP
        opp2 = runner.place_on_field(2, ["ST1-06"])  # 6000 DP
        opp3 = runner.place_on_field(2, ["ST1-06"])  # 6000 DP

        game = runner.game
        op_effect = _find_on_play_effect(guardromon)
        assert op_effect is not None

        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        op_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # Exactly ONE opponent should be at 3000 DP; the others unaffected
        dps = sorted([opp1.dp, opp2.dp, opp3.dp])
        assert dps == [3000, 6000, 6000], (
            f"-3000 DP must apply to exactly 1 target; got DPs={dps}. "
            "If all went to 3000, the modifier leaked to all permanents "
            "(missing target-equality guard)."
        )

    def test_on_play_dp_debuff_also_applies_to_own_friendly_digimon_check(self, debug_runner):
        """CRITICAL leak guard: -3000 DP must not leak to friendly Digimon either."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-06"] * 50,
            initial_memory=10,
        )
        guardromon = runner.place_on_field(1, ["BT22-056"])
        friend = runner.place_on_field(1, ["ST1-03"])  # Agumon 2000 DP
        opp = runner.place_on_field(2, ["ST1-06"])  # Coredramon 6000 DP

        friend_base_dp = friend.dp

        game = runner.game
        op_effect = _find_on_play_effect(guardromon)
        assert op_effect is not None

        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        op_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # Friend must NOT have been debuffed; Guardromon itself must NOT be debuffed
        assert friend.dp == friend_base_dp, (
            f"Friendly Agumon should have {friend_base_dp} DP, got {friend.dp}. "
            "The -3000 DP leaked to friendly permanents."
        )
        # opp should be at 3000
        assert opp.dp == 3000

    def test_on_play_dp_debuff_expires_at_end_of_turn(self, debug_runner):
        """The -3000 DP debuff must be cleared at end of turn."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-06"] * 50,
            initial_memory=10,
        )
        guardromon = runner.place_on_field(1, ["BT22-056"])
        opp = runner.place_on_field(2, ["ST1-06"])  # 6000 DP

        game = runner.game
        op_effect = _find_on_play_effect(guardromon)
        assert op_effect is not None

        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        op_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)
        assert opp.dp == 3000

        # End turn — modifiers with 'end_of_turn' expiry are cleared
        game.modifiers.clear_expiry('end_of_turn')
        assert opp.dp == 6000, (
            f"After end-of-turn cleanup, DP should revert to 6000, got {opp.dp}"
        )

    # ── Same-level stack gate ────────────────────────────────────────

    def test_no_de_digivolve_when_stack_has_no_same_level_pair(self, debug_runner):
        """If stack has no 2+ same-level cards, NO de-digivolve happens."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03", "ST1-06"] * 25,
            initial_memory=10,
        )
        # Guardromon solo stack (just its own top card, Lv4). No same-level pair.
        guardromon = runner.place_on_field(1, ["BT22-056"])

        # Opponent stack: Lv3 Agumon -> Lv4 Coredramon (no same-level pair on opp either)
        opp = runner.place_on_field(2, ["ST1-03", "ST1-06"])
        initial_opp_stack_size = len(opp.card_sources)
        assert initial_opp_stack_size == 2

        game = runner.game
        op_effect = _find_on_play_effect(guardromon)
        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        op_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # Opponent stack must NOT have been de-digivolved
        assert len(opp.card_sources) == initial_opp_stack_size, (
            f"Opponent stack was de-digivolved despite no same-level pair in Guardromon's stack. "
            f"Sources: {[cs.c_entity_base.card_id if cs.c_entity_base else '?' for cs in opp.card_sources]}"
        )

    def test_de_digivolve_when_stack_has_same_level_pair(self, debug_runner):
        """If stack has 2+ cards at the same level, de-digivolve 1 opponent Digimon."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03", "ST1-06"] * 25,
            initial_memory=10,
        )
        # Stack: [Agumon Lv3, Dracomon Lv3, Guardromon Lv4]
        # => two Lv3 sources — triggers de-digivolve.
        guardromon = runner.place_on_field(1, ["ST1-03", "ST1-04", "BT22-056"])
        # Opponent: Lv3 Agumon under Lv4 Coredramon — de-digivolve should trash Coredramon, leaving Agumon
        opp = runner.place_on_field(2, ["ST1-03", "ST1-06"])
        assert len(opp.card_sources) == 2

        game = runner.game
        op_effect = _find_on_play_effect(guardromon)
        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        op_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # Opponent's stack should have shrunk by 1 (top Coredramon removed)
        assert len(opp.card_sources) == 1, (
            f"De-Digivolve 1 should remove the top card. "
            f"Stack: {[cs.c_entity_base.card_id if cs.c_entity_base else '?' for cs in opp.card_sources]}"
        )
        # Remaining top should be the base ST1-03 (Agumon)
        assert opp.top_card.c_entity_base.card_id == "ST1-03"

    def test_same_level_pair_counts_top_card(self, debug_runner):
        """Same-level check counts the top card too (per C# StackCards semantics).

        Two Lv4 sources stacked (the egg/whatever at Lv4 + Guardromon top Lv4)
        should trigger de-digivolve.
        """
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03", "ST1-06"] * 25,
            initial_memory=10,
        )
        # Stack: [Coredramon Lv4, Guardromon Lv4] => 2 Lv4 cards
        guardromon = runner.place_on_field(1, ["ST1-06", "BT22-056"])
        opp = runner.place_on_field(2, ["ST1-03", "ST1-06"])
        assert len(opp.card_sources) == 2

        game = runner.game
        op_effect = _find_on_play_effect(guardromon)
        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        op_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        assert len(opp.card_sources) == 1, (
            f"Stack has 2 Lv4 cards — de-digivolve should trigger. "
            f"Opp stack size: {len(opp.card_sources)}"
        )

    def test_de_digivolve_trashes_top_card_to_opp_trash(self, debug_runner):
        """De-Digivolved card goes to the OPPONENT's trash (owner of the stack)."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03", "ST1-06"] * 25,
            initial_memory=10,
        )
        guardromon = runner.place_on_field(1, ["ST1-03", "ST1-04", "BT22-056"])
        opp = runner.place_on_field(2, ["ST1-03", "ST1-06"])

        game = runner.game
        opp_trash_before = len(game.player2.trash_cards)

        op_effect = _find_on_play_effect(guardromon)
        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        op_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        opp_trash_after = len(game.player2.trash_cards)
        assert opp_trash_after == opp_trash_before + 1, (
            f"Expected 1 new card in opp trash after de-digivolve. "
            f"Before: {opp_trash_before}, After: {opp_trash_after}"
        )
        # Trashed card should be the Coredramon (ST1-06) that was on top
        trashed_ids = [c.c_entity_base.card_id for c in game.player2.trash_cards
                       if c.c_entity_base]
        assert "ST1-06" in trashed_ids, (
            f"Coredramon (ST1-06) should be in opponent's trash. Got: {trashed_ids}"
        )

    # ── C2: [When Digivolving] has same clauses ──────────────────────

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Effect list should have a When Digivolving instance separate from On Play."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        guardromon = runner.place_on_field(1, ["BT22-056"])

        op_effect = _find_on_play_effect(guardromon)
        wd_effect = _find_wd_effect(guardromon)

        assert op_effect is not None, "Should have an On Play effect"
        assert wd_effect is not None, "Should have a When Digivolving effect"
        assert op_effect is not wd_effect, (
            "On Play and When Digivolving must be SEPARATE effect instances"
        )

    def test_wd_applies_minus_3000_to_selected_opponent(self, debug_runner):
        """When Digivolving selects 1 opponent Digimon and applies -3000 DP."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-06"] * 50,
            initial_memory=10,
        )
        guardromon = runner.place_on_field(1, ["BT22-056"])
        opp = runner.place_on_field(2, ["ST1-06"])  # 6000 DP

        game = runner.game
        wd_effect = _find_wd_effect(guardromon)
        assert wd_effect is not None

        ctx = {'game': game, 'player': game.player1, 'permanent': guardromon}
        wd_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        assert opp.dp == 3000, (
            f"After When Digivolving, opp should be 3000 DP. Got {opp.dp}"
        )

    # ── C3: Inherited [Opponent's Turn] +2000 DP ─────────────────────

    def test_inherited_effect_exists(self, debug_runner):
        """Should have an inherited dp_modifier=2000 effect."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        perm = runner.place_on_field(1, ["BT22-056"])
        ess = _find_inherited_dp_effect(perm)
        assert ess is not None, (
            "BT22-056 should have an inherited effect with dp_modifier=2000"
        )
        assert getattr(ess, 'is_inherited_effect', False) is True

    def test_inherited_dp_condition_gated_on_opponent_turn(self, debug_runner):
        """The +2000 DP inherited effect must only pass its condition on the opponent's turn."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
            first_player=1,  # p1's turn
        )
        perm = runner.place_on_field(1, ["BT22-056"])
        ess = _find_inherited_dp_effect(perm)
        assert ess is not None

        # P1's turn = OWNER's turn. Condition must return False.
        game = runner.game
        assert game.player1.is_my_turn is True
        ctx = {'permanent': perm}
        assert ess.can_use_condition(ctx) is False, (
            "Inherited +2000 DP must NOT apply on owner's turn"
        )

        # Flip to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert ess.can_use_condition(ctx) is True, (
            "Inherited +2000 DP must apply on opponent's turn"
        )

    def test_inherited_dp_applied_as_aura_on_opp_turn(self, debug_runner):
        """When BT22-056 is UNDER another Digimon (in the stack), the host
        digivolved on top of it gets +2000 DP on the opponent's turn."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
            first_player=1,
        )
        # Stack: [Guardromon Lv4 (inherited), Coredramon Lv4 top]
        perm = runner.place_on_field(1, ["BT22-056", "ST1-06"])
        game = runner.game
        # Base DP = Coredramon 6000
        assert perm.dp == 6000, (
            f"Base DP should be Coredramon 6000 on owner's turn "
            f"(inherited effect inactive), got {perm.dp}"
        )

        # Flip turn to opponent
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        # Coredramon should now show +2000 DP = 8000 from Guardromon inherited
        assert perm.dp == 8000, (
            f"On opponent's turn, Coredramon(+Guardromon inherited) should be "
            f"8000 DP (6000 + 2000). Got {perm.dp}."
        )

    def test_inherited_dp_not_applied_on_own_turn(self, debug_runner):
        """On owner's turn, the inherited +2000 DP is NOT applied."""
        runner = debug_runner(
            deck1=["BT22-056"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
            first_player=1,
        )
        # Stack with Guardromon inherited under Coredramon
        perm = runner.place_on_field(1, ["BT22-056", "ST1-06"])
        game = runner.game
        assert game.player1.is_my_turn is True
        assert perm.dp == 6000, (
            f"On owner's turn, inherited effect should NOT apply. Got {perm.dp}"
        )

    # ── Alt-Digi: Lv.3 CS for cost 2 ─────────────────────────────────

    def test_alt_digivolve_attributes_present(self, debug_runner):
        """Should expose alt-digi attributes for cost 2, level 3."""
        runner = debug_runner(
            deck1=["BT22-056"] * 50,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        # Grab a fresh Guardromon card source from hand or deck
        p1 = runner.game.player1
        cs = None
        for c in p1.library_cards + p1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT22-056":
                cs = c
                break
        assert cs is not None
        effects = cs.effect_list(EffectTiming.NoTiming)
        alt_digi = None
        for eff in effects:
            if getattr(eff, '_alt_digi_level', None) is not None:
                alt_digi = eff
                break
        assert alt_digi is not None, "Should have an alt-digi factory effect"
        assert alt_digi._alt_digi_level == 3
        assert alt_digi._alt_digi_cost == 2
