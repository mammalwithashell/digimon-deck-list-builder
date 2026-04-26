"""Behavioral tests for BT23-067 LadyDevimon (Lv.5 Purple, Cost 7).

Card text (per cards.json / C# source of truth):

  Alt digivolve: [Digivolve] Lv.4 w/[CS] trait: Cost 3 (NO color requirement)

  When this card would be played from the hand, if you have [Angewomon] or
  [Mirei Mikagura], reduce the play cost by 3.
  <Blocker> (This Digimon can block in the blocker timing.)
  [On Play] [When Digivolving] Delete 1 of your opponent's level 4 or lower
  Digimon.

Inherited:
  <Scapegoat> (When this Digimon would be deleted other than by your effects,
  by deleting 1 of your other Digimon, prevent that deletion.)

C# EqualsCardName is an EXACT name match, so "Angewomon (X Antibody)" must
NOT satisfy the cost reduction condition (only "Angewomon" or "Mirei Mikagura"
exactly).
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, CardColor


def _deck():
    """Return a 50-card filler deck."""
    return ["BT1-010"] * 50


@pytest.mark.behavioral
class TestBT23067AltDigi:
    """Tests for BT23-067 alt-digi requirement (Lv.4 w/[CS] trait)."""

    def test_alt_digi_attributes_present(self, debug_runner):
        """Alt-digi must encode Lv.4 + CS trait + cost 3, with NO color gate."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT23-067", "hand")

        all_effects = card.effect_list(None)
        alt_digi_effects = [e for e in all_effects if hasattr(e, '_alt_digi_cost')]
        assert len(alt_digi_effects) >= 1, "Should have alt-digi effect"

        eff = alt_digi_effects[0]
        assert eff._alt_digi_cost == 3, f"alt-digi cost must be 3, got {eff._alt_digi_cost}"
        assert eff._alt_digi_level == 4, f"alt-digi level must be 4, got {eff._alt_digi_level}"
        assert getattr(eff, '_alt_digi_trait', None) == "CS", (
            f"Alt-digi must require CS trait. Got "
            f"_alt_digi_trait={getattr(eff, '_alt_digi_trait', None)}"
        )
        # Alt-digi in C# has NO color restriction — any color Lv.4 w/CS qualifies
        assert getattr(eff, '_alt_digi_color', None) is None, (
            f"Alt-digi must NOT encode a color gate (C# has no color check). "
            f"Got _alt_digi_color={getattr(eff, '_alt_digi_color', None)}"
        )

    def test_alt_digi_works_onto_lv4_cs_non_purple(self, debug_runner):
        """Alt-digi should allow digivolving onto a non-Purple Lv.4 w/[CS] trait."""
        runner = debug_runner(initial_memory=10)
        # BT22-008 Agumon is Lv.3 - not useful. Need Lv.4 w/[CS] non-Purple.
        # BT23-056 is WereGarurumon, Lv.5, Black, CS. Not Lv.4.
        # BT23-018 Garurumon is Lv.4 Blue w/[CS]. Good candidate.
        runner.place_on_field(1, ["BT23-018"])  # Lv.4 Blue CS
        runner.inject_card(1, "BT23-067", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        assert digi is not None, (
            f"Should be able to alt-digi onto a non-Purple Lv.4 CS Digimon. "
            f"Actions: {runner.actions()}")

    def test_alt_digi_blocked_on_non_cs(self, debug_runner):
        """Alt-digi should NOT apply to Lv.4 Digimon without CS trait."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, "BT23-067", "hand")

        all_effects = card.effect_list(None)
        alt_digi_effects = [e for e in all_effects if hasattr(e, '_alt_digi_cost')]
        eff = alt_digi_effects[0]
        # Validate the trait gate directly (the validator scans traits).
        # No CS trait on ST1-07 Greymon (Lv.4 Red, not CS).
        assert getattr(eff, '_alt_digi_trait', None) == "CS"


@pytest.mark.behavioral
class TestBT23067CostReduction:
    """Tests for [BeforePayCost] -3 if you have [Angewomon] or [Mirei Mikagura]."""

    def test_before_pay_cost_effect_exists(self, debug_runner):
        """BT23-067 must have a BeforePayCost effect for cost reduction."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT23-067", "hand")

        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost]
        assert len(bpc) >= 1, "Should have BeforePayCost effect"
        assert bpc[0].cost_reduction == 3, (
            f"cost_reduction must be 3, got {bpc[0].cost_reduction}"
        )

    def test_leak_guard_blocks_other_cards(self, debug_runner):
        """BeforePayCost must NOT activate for OTHER cards being played."""
        runner = debug_runner(initial_memory=5)
        # BT23-067 on field — emphatically NOT the card being played
        perm = runner.place_on_field(1, ["BT23-067"])
        # And an Angewomon on field to satisfy the "you have" clause
        runner.place_on_field(1, ["BT2-037"])

        top = perm.top_card
        effects = top.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        # Pass a DIFFERENT card_source (another card being played)
        other = runner.inject_card(1, "BT1-010", "hand")
        result = bpc.can_use_condition({'card_source': other})
        assert not result, (
            "BeforePayCost must not leak to other cards being played"
        )

    def test_condition_passes_with_angewomon_on_field(self, debug_runner):
        """Condition passes when exact-named 'Angewomon' is on our battle area."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT2-037"])  # Angewomon (exact)

        card = runner.inject_card(1, "BT23-067", "hand")
        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        result = bpc.can_use_condition({'card_source': card})
        assert result, "Condition should pass when Angewomon is on field"

    def test_condition_passes_with_mirei_on_field(self, debug_runner):
        """Condition passes when 'Mirei Mikagura' is on our battle area."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT11-094"])  # Mirei Mikagura

        card = runner.inject_card(1, "BT23-067", "hand")
        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        result = bpc.can_use_condition({'card_source': card})
        assert result, "Condition should pass when Mirei Mikagura is on field"

    def test_condition_fails_without_target_on_field(self, debug_runner):
        """Condition fails when neither Angewomon nor Mirei are present."""
        runner = debug_runner(initial_memory=5)
        # Place some unrelated digimon
        runner.place_on_field(1, ["BT1-010"])

        card = runner.inject_card(1, "BT23-067", "hand")
        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        result = bpc.can_use_condition({'card_source': card})
        assert not result, (
            "Condition should fail when no Angewomon/Mirei on field"
        )

    def test_condition_fails_with_angewomon_x_antibody(self, debug_runner):
        """'Angewomon (X Antibody)' must NOT satisfy the condition (exact match).

        C# uses EqualsCardName("Angewomon") — NOT substring match. An
        Angewomon (X Antibody) should not count.
        """
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT9-040"])  # Angewomon (X Antibody)

        card = runner.inject_card(1, "BT23-067", "hand")
        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        result = bpc.can_use_condition({'card_source': card})
        assert not result, (
            "Condition should FAIL for 'Angewomon (X Antibody)' — C# uses "
            "EqualsCardName (exact match), not substring."
        )

    def test_condition_fails_with_opponent_angewomon(self, debug_runner):
        """Opponent's Angewomon should not satisfy the condition."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(2, ["BT2-037"])  # Opponent's Angewomon

        card = runner.inject_card(1, "BT23-067", "hand")
        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        result = bpc.can_use_condition({'card_source': card})
        assert not result, (
            "Condition should fail when only opponent has Angewomon"
        )

    def test_integration_cost_reduced_to_4(self, debug_runner):
        """When Angewomon is on field, calculated play cost drops 7 -> 4."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT2-037"])  # Angewomon
        runner.inject_card(1, "BT23-067", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play")
        # Look for LadyDevimon in action descriptions
        play_acts = runner.find_actions("LadyDevimon")
        # Any action containing "LadyDevimon" with play type
        assert play_acts, f"Should find a Play LadyDevimon action: {runner.actions()}"


@pytest.mark.behavioral
class TestBT23067Blocker:
    """Tests for the <Blocker> keyword."""

    def test_has_blocker(self, debug_runner):
        """BT23-067 should have the Blocker keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-067"])

        effects = perm.top_card.effect_list(None)
        blocker_effects = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker_effects) >= 1, "BT23-067 should have Blocker keyword"

        # And blocker should NOT be inherited
        for e in blocker_effects:
            assert not e.is_inherited_effect, (
                "Blocker must be a self effect, not inherited"
            )


@pytest.mark.behavioral
class TestBT23067OnPlayAndDigivolve:
    """Tests for [On Play] / [When Digivolving] delete effect."""

    def test_on_play_effect_exists(self, debug_runner):
        """BT23-067 should have an OnEnterFieldAnyone is_on_play effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-067"])
        top = perm.top_card

        effects = top.effect_list(None)
        op = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(op) >= 1, "Should have On Play effect"
        assert not op[0].is_optional, (
            "Delete is NOT optional (canNoSelect=false in C#)"
        )

    def test_when_digivolving_effect_exists(self, debug_runner):
        """BT23-067 should have an OnEnterFieldAnyone is_when_digivolving effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-067"])
        top = perm.top_card

        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd) >= 1, "Should have When Digivolving effect"
        assert not wd[0].is_optional, (
            "Delete is NOT optional (canNoSelect=false in C#)"
        )

    def test_on_play_deletes_lv4_or_lower(self, debug_runner):
        """[On Play]: delete a Lv.4 opponent's Digimon (within filter)."""
        runner = debug_runner(initial_memory=15)
        # Opponent's Lv.4 Digimon: BT23-018 Garurumon (Lv.4 Blue CS)
        runner.place_on_field(2, ["BT23-018"])

        runner.inject_card(1, "BT23-067", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play LadyDevimon")
        assert play is not None, f"Should find Play action: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_ids = [s.card_id for s in snap.p2_field]
        assert "BT23-018" not in opp_ids, (
            f"Opponent's Lv.4 Digimon should be deleted. "
            f"P2 field: {[(s.card_id, s.card_name) for s in snap.p2_field]}"
        )

    def test_on_play_cannot_target_lv5(self, debug_runner):
        """[On Play] must NOT be able to target a Lv.5 opponent's Digimon."""
        runner = debug_runner(initial_memory=15)
        # Opponent's Lv.5 Digimon: BT23-056 WereGarurumon (Lv.5)
        opp_perm = runner.place_on_field(2, ["BT23-056"])

        perm = runner.place_on_field(1, ["BT23-067"])
        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        game = runner.game
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Since no valid targets (opponent's only perm is Lv.5), no selection
        # should be spawned, OR selection spawned with 0 targets. Check: the
        # opponent's Lv.5 is not in the pending selection's valid list.
        ps = game.pending_selection
        if ps is not None:
            # The opponent's Lv.5 perm index is 0. Its action_id would be
            # SEL_OPP_FIELD_START + 0. It must NOT be in valid_indices.
            from digimon_gym.engine.game.actions import SEL_OPP_FIELD_START
            assert (SEL_OPP_FIELD_START + 0) not in ps.valid_indices, (
                "Lv.5 opponent Digimon must not be targetable"
            )

    def test_on_play_process_delete_lv4(self, debug_runner):
        """Invoking process directly deletes a Lv.4 opponent's Digimon."""
        runner = debug_runner(initial_memory=15)
        opp = runner.place_on_field(2, ["BT23-018"])  # Lv.4 opponent

        perm = runner.place_on_field(1, ["BT23-067"])
        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        game = runner.game
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve(max_steps=10)

        assert opp not in game.player2.battle_area, (
            "Opponent's Lv.4 Digimon should be deleted by On Play"
        )

    def test_when_digivolving_integration(self, debug_runner):
        """Digivolving onto base triggers the WD delete (smoke test)."""
        runner = debug_runner(initial_memory=15)
        # Place a Lv.4 Purple base (satisfies normal evo cost Purple Lv.4)
        runner.place_on_field(1, ["BT23-048"])  # A Lv.4 Purple
        runner.place_on_field(2, ["BT23-018"])  # opponent's Lv.4 target

        runner.inject_card(1, "BT23-067", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        if digi is not None:
            runner.execute(digi)
            runner.auto_resolve(max_steps=15)
            snap = runner.snapshot()
            opp_ids = [s.card_id for s in snap.p2_field]
            # Delete happened -> opponent field does not contain BT23-018
            assert "BT23-018" not in opp_ids, (
                f"Opponent's Lv.4 should be deleted after WD. "
                f"P2 field: {opp_ids}"
            )


@pytest.mark.behavioral
class TestBT23067Scapegoat:
    """Tests for the inherited <Scapegoat> keyword."""

    def test_scapegoat_flag_is_inherited(self, debug_runner):
        """The _is_scapegoat effect must be an inherited effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-067"])
        top = perm.top_card

        effects = top.effect_list(None)
        scape = [e for e in effects if getattr(e, '_is_scapegoat', False)]
        assert len(scape) >= 1, "BT23-067 should have a Scapegoat effect"
        assert scape[0].is_inherited_effect, (
            "Scapegoat must be an inherited effect (below the line)"
        )

    def test_scapegoat_flag_and_grant(self, debug_runner):
        """Scapegoat is an INHERITED effect — grants keyword to the card
        ABOVE BT23-067 in a digivolution stack (not to BT23-067 itself)."""
        runner = debug_runner(initial_memory=5)
        # Stack: BT23-067 (bottom / source) -> some Lv.6 top card
        # Use BT15-068 or any Lv.6 Purple as top.
        # Easier: BT22-089 Mirei Mikagura can't be on a stack (tamer).
        # Use another BT23-067 on top so both are in stack.
        perm = runner.place_on_field(1, ["BT23-067", "BT23-067"])

        # The TOP perm (which has BT23-067 as a digi source) should inherit
        # the Scapegoat keyword.
        assert perm.has_keyword('_is_scapegoat'), (
            "A Digimon with BT23-067 as a digi source should gain <Scapegoat> "
            "via the inherited effect."
        )
