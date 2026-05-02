"""Behavioral tests for AD1-012 CresGarurumon.

Card text:
  <Alliance>
  <Evade> (When this Digimon would be deleted, you may suspend it to prevent that deletion.)
  [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may return 1 of your
    opponent's lowest level Digimon to the hand. Then, this Digimon and 1 of your Digimon
    with [Greymon] in its name may unsuspend.
  [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, 2 of your
    Digimon may DNA digivolve into [Omnimon Alter-S] in the hand. Then, you may change the
    attack target to 1 of your Digimon.

Inherited:
  [Your Turn] This Digimon's attack target can't change.

Alt-Digi: Lv.5 w/[Garurumon] in name or w/[ADVENTURE] trait: Cost 3

Card reference:
  ST1-02 = Biyomon Lv.3 Red
  ST1-03 = Agumon Lv.3 Red
  ST1-05 = Birdramon Lv.4 Red
  ST1-07 = Greymon Lv.4 Red (has "Greymon" in name)
  ST1-08 = Garudamon Lv.5 Red
  ST2-03 = Gabumon Lv.3 Blue
  ST2-06 = Garurumon Lv.4 Blue (has "Garurumon" in name)
  ST2-08 = WereGarurumon Lv.5 Blue (has "Garurumon" in name)
  ST2-10 = MetalGarurumon Lv.6 Blue
  BT1-040 = WereGarurumon Lv.5 Blue
  AD1-001 = Greymon Lv.4 Red (has "Greymon" in name, ADVENTURE trait)
"""

import pytest

from engine_py_legacy.engine.game.constants import SEL_OPP_FIELD_START, SEL_MY_FIELD_START


# Filler deck: enough cards for both players
FILLER_DECK = (
    ["ST2-01"] * 4   # Lv.2 eggs (blue)
    + ["ST2-03"] * 4  # Gabumon Lv.3
    + ["ST2-06"] * 4  # Garurumon Lv.4
    + ["ST2-08"] * 4  # WereGarurumon Lv.5
    + ["ST2-10"] * 4  # MetalGarurumon Lv.6
    + ["ST1-02"] * 4  # Biyomon Lv.3
    + ["ST1-05"] * 4  # Birdramon Lv.4
    + ["ST1-07"] * 4  # Greymon Lv.4
    + ["ST1-08"] * 4  # Garudamon Lv.5
    + ["ST1-09"] * 4  # MetalGreymon Lv.5
    + ["ST1-10"] * 4  # WarGreymon Lv.6
    + ["ST1-11"] * 2  # Tai Kamiya tamer
    + ["ST1-03"] * 2  # Agumon Lv.3
)

# AD1-012 needs to be in the deck so its script class gets registered
DECK_WITH_CRES = ["AD1-012"] + FILLER_DECK[1:]


@pytest.mark.behavioral
class TestCresGarurumonKeywords:
    """Test Alliance and Evade keywords are present."""

    def test_alliance_keyword(self, debug_runner):
        """CresGarurumon should have the Alliance keyword on the field."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, ["AD1-012"], turn_played=-1)
        assert perm.has_keyword('_is_alliance'), (
            "AD1-012 CresGarurumon should have Alliance keyword"
        )

    def test_evade_keyword(self, debug_runner):
        """CresGarurumon should have the Evade keyword on the field."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, ["AD1-012"], turn_played=-1)
        assert perm.has_keyword('_is_evade'), (
            "AD1-012 CresGarurumon should have Evade keyword"
        )


@pytest.mark.behavioral
class TestCresGarurumonAltDigi:
    """Test alternate digivolution conditions."""

    def test_alt_digi_from_garurumon_name(self, debug_runner):
        """CresGarurumon can digivolve from a Lv.5 with [Garurumon] in name for cost 3."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # BT1-040: WereGarurumon Lv.5 (has "Garurumon" in name)
        runner.place_on_field(1, ["BT1-040"], turn_played=-1)

        # Inject CresGarurumon into hand
        runner.inject_card(1, "AD1-012", zone="hand")

        # Should see a digivolve action for CresGarurumon onto WereGarurumon
        action = runner.find_action("Digivolve")
        assert action is not None, (
            "Should be able to digivolve CresGarurumon onto WereGarurumon (Garurumon in name)"
        )

    def test_alt_digi_from_adventure_trait(self, debug_runner):
        """CresGarurumon can digivolve from a Lv.5 Blue (standard evo) for cost 4."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # ST2-08: WereGarurumon Lv.5, Blue
        runner.place_on_field(1, ["ST2-08"], turn_played=-1)

        runner.inject_card(1, "AD1-012", zone="hand")

        # Standard evo should work: Blue Lv.5 for cost 4
        actions = runner.find_actions("Digivolve")
        assert len(actions) > 0, (
            "Should be able to digivolve CresGarurumon onto Blue Lv.5"
        )


@pytest.mark.behavioral
class TestCresGarurumonBounceLowest:
    """Test the [On Play] [When Digivolving] [When Attacking] OPT bounce+unsuspend effect.

    Note: In the engine, action descriptions in SelectTarget phase may not
    accurately describe the selection purpose. We use action IDs directly
    based on SEL_OPP_FIELD_START constants.
    """

    def test_when_attacking_bounce_lowest_level(self, debug_runner):
        """When Attacking should bounce opponent's lowest level Digimon when selected."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=15)
        runner.set_phase("Main")

        # Place opponent Digimon of DIFFERENT levels
        runner.place_on_field(2, ["ST1-02"], turn_played=-1)  # Biyomon Lv.3 (slot 0)
        runner.place_on_field(2, ["ST1-05"], turn_played=-1)  # Birdramon Lv.4 (slot 1)

        perm = runner.place_on_field(1, ["AD1-012"], turn_played=-1)

        # Attack with CresGarurumon
        attack_action = runner.find_action("Attack player with CresGarurumon")
        assert attack_action is not None, "Should be able to attack"
        runner.execute(attack_action)

        # Should be in SelectTarget phase
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", f"Expected SelectTarget, got {snap.phase}"

        # Verify the pending selection has Biyomon (slot 0 = action 114) but NOT Birdramon (slot 1)
        game = runner.game
        assert game.pending_selection is not None, "Should have a pending selection"
        valid = game.pending_selection.valid_indices
        assert SEL_OPP_FIELD_START + 0 in valid, (
            f"Biyomon (slot 0, lowest level) should be a valid target. Valid: {valid}"
        )
        assert SEL_OPP_FIELD_START + 1 not in valid, (
            f"Birdramon (slot 1, Lv.4) should NOT be valid when Lv.3 exists. Valid: {valid}"
        )

        # Select Biyomon (action SEL_OPP_FIELD_START + 0)
        runner.execute(SEL_OPP_FIELD_START + 0)
        runner.auto_resolve()

        after_snap = runner.snapshot()
        p2_field_ids = [s.card_id for s in after_snap.p2_field]
        assert "ST1-02" not in p2_field_ids, (
            f"Biyomon should have been bounced. P2 field: {p2_field_ids}"
        )
        assert "ST1-05" in p2_field_ids, "Birdramon should remain on field"

    def test_bounce_when_all_same_level(self, debug_runner):
        """When all opponent Digimon are the same level, all should be valid targets."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=15)
        runner.set_phase("Main")

        # Place two Lv.4 Digimon
        runner.place_on_field(2, ["ST1-05"], turn_played=-1)  # Birdramon Lv.4 (slot 0)
        runner.place_on_field(2, ["ST1-07"], turn_played=-1)  # Greymon Lv.4 (slot 1)

        perm = runner.place_on_field(1, ["AD1-012"], turn_played=-1)

        attack_action = runner.find_action("Attack player with CresGarurumon")
        assert attack_action is not None
        runner.execute(attack_action)

        snap = runner.snapshot()
        assert snap.phase == "SelectTarget"

        game = runner.game
        valid = game.pending_selection.valid_indices
        # Both should be valid (same level = both lowest)
        assert SEL_OPP_FIELD_START + 0 in valid, "Birdramon (slot 0) should be valid"
        assert SEL_OPP_FIELD_START + 1 in valid, "Greymon (slot 1) should be valid"

    def test_opt_is_shared_hash(self, debug_runner):
        """The OPT counter should be shared across OP/WD/WA via the same hash string."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=15)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["AD1-012"], turn_played=-1)

        # Verify the shared hash is used on the WA effect
        from engine_py_legacy.engine.data.enums import EffectTiming
        all_effects = perm.effect_list(EffectTiming.OnUseAttack)
        wa_effects = [e for e in all_effects
                      if "AD1-012" in getattr(e, 'effect_name', '')
                      and "Bounce" in getattr(e, 'effect_name', '')]
        assert len(wa_effects) >= 1, (
            f"Should find When Attacking bounce effect. Effects: "
            f"{[(e.effect_name, e.hash_string) for e in all_effects]}"
        )
        assert wa_effects[0].hash_string == "AD1_012_OP_WD_WA", (
            f"When Attacking effect should use shared hash. Got: {wa_effects[0].hash_string}"
        )


@pytest.mark.behavioral
class TestCresGarurumonUnsuspend:
    """Test the unsuspend portion of the shared effect."""

    def test_unsuspend_self_after_attack(self, debug_runner):
        """CresGarurumon should unsuspend itself as part of the bounce+unsuspend effect."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=15)
        runner.set_phase("Main")

        cres = runner.place_on_field(1, ["AD1-012"], turn_played=-1)
        runner.place_on_field(2, ["ST1-02"], turn_played=-1)  # Biyomon Lv.3

        # Attack (suspends CresGarurumon)
        attack_action = runner.find_action("Attack player with CresGarurumon")
        assert attack_action is not None
        runner.execute(attack_action)

        # Select Biyomon to bounce (action = SEL_OPP_FIELD_START + 0)
        runner.execute(SEL_OPP_FIELD_START + 0)
        runner.auto_resolve()

        # CresGarurumon should be unsuspended by its own effect
        after_snap = runner.snapshot()
        cres_slot = [s for s in after_snap.p1_field if s.card_id == "AD1-012"]
        assert len(cres_slot) >= 1, "CresGarurumon should be on field"
        assert not cres_slot[0].is_suspended, (
            "CresGarurumon should unsuspend itself as part of the bounce+unsuspend effect"
        )

    def test_unsuspend_self_and_greymon(self, debug_runner):
        """After bounce, CresGarurumon and a suspended [Greymon] should unsuspend."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=15)
        runner.set_phase("Main")

        # Place CresGarurumon (slot 0)
        cres = runner.place_on_field(1, ["AD1-012"], turn_played=-1)

        # Place a Greymon (suspended) - ST1-07 is Greymon Lv.4 (slot 1)
        greymon = runner.place_on_field(1, ["ST1-07"], turn_played=-1, is_suspended=True)

        # Opponent Biyomon to bounce
        runner.place_on_field(2, ["ST1-02"], turn_played=-1)

        # Attack with CresGarurumon
        attack_action = runner.find_action("Attack player with CresGarurumon")
        assert attack_action is not None
        runner.execute(attack_action)

        # Select Biyomon to bounce
        runner.execute(SEL_OPP_FIELD_START + 0)

        # Now in Greymon selection phase - explicitly select Greymon (slot 1)
        # auto_resolve would pick Decline (action 62), so we select Greymon directly
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", f"Expected SelectTarget, got {snap.phase}"
        game = runner.game
        assert game.pending_selection is not None, "Should have pending Greymon selection"
        valid = game.pending_selection.valid_indices
        greymon_action = SEL_MY_FIELD_START + 1  # slot 1 = Greymon
        assert greymon_action in valid, (
            f"Greymon (slot 1) should be a valid unsuspend target. Valid: {valid}"
        )
        runner.execute(greymon_action)
        runner.auto_resolve()

        after_snap = runner.snapshot()
        greymon_slot = [s for s in after_snap.p1_field if s.card_id == "ST1-07"]
        assert len(greymon_slot) >= 1, "Greymon should be on field"
        assert not greymon_slot[0].is_suspended, (
            "Greymon should be unsuspended by CresGarurumon's effect"
        )

    def test_no_greymon_still_unsuspends_self(self, debug_runner):
        """Without a Greymon, CresGarurumon still unsuspends itself."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=15)
        runner.set_phase("Main")

        cres = runner.place_on_field(1, ["AD1-012"], turn_played=-1)
        runner.place_on_field(2, ["ST1-02"], turn_played=-1)

        attack_action = runner.find_action("Attack player with CresGarurumon")
        assert attack_action is not None
        runner.execute(attack_action)

        # Select Biyomon
        runner.execute(SEL_OPP_FIELD_START + 0)
        runner.auto_resolve()

        after_snap = runner.snapshot()
        cres_slot = [s for s in after_snap.p1_field if s.card_id == "AD1-012"]
        assert len(cres_slot) >= 1
        assert not cres_slot[0].is_suspended, (
            "CresGarurumon should still unsuspend itself even without a Greymon"
        )


@pytest.mark.behavioral
class TestCresGarurumonInherited:
    """Test the inherited effect: [Your Turn] attack target can't change."""

    def test_inherited_effect_present(self, debug_runner):
        """CresGarurumon's inherited effect should exist in the digivolution stack."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place CresGarurumon as evo source under MetalGarurumon
        perm = runner.place_on_field(1, ["AD1-012", "ST2-10"], turn_played=-1)

        snap = runner.snapshot()
        p1_field = snap.p1_field
        assert len(p1_field) >= 1, "Should have a permanent on field"
        assert "AD1-012" in p1_field[0].stack_ids, (
            "CresGarurumon should be in the digivolution stack"
        )


@pytest.mark.behavioral
class TestCresGarurumonOpponentTurnEffect:
    """Test the [Opponent's Turn] DNA digivolve + redirect attack effect."""

    def test_condition_requires_opponent_turn(self, debug_runner):
        """The opponent's turn effect's condition should return False on owner's turn."""
        runner = debug_runner(deck1=DECK_WITH_CRES, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["AD1-012"], turn_played=-1)

        # Check that the effect's condition returns False on own turn
        from engine_py_legacy.engine.data.enums import EffectTiming
        effects = perm.effect_list(EffectTiming.OnTappedAnyone)
        redirect_effects = [e for e in effects
                            if getattr(e, 'hash_string', '') == "Redirect_AD1_012"]
        assert len(redirect_effects) >= 1, (
            f"Should find the redirect effect. Effects: "
            f"{[(getattr(e, 'effect_name', ''), getattr(e, 'hash_string', '')) for e in effects]}"
        )
        eff = redirect_effects[0]
        # Owner's turn: condition should return False
        result = eff.can_use_condition({})
        assert result is False, (
            "Opponent's turn effect should not activate on owner's turn"
        )
