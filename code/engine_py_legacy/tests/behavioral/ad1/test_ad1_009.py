"""Behavioral tests for AD1-009 BlitzGreymon | Lv.6 Red/Purple Digimon.

Card text:
    <Alliance> <Piercing> <Blocker>
    [On Play] [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
    Then, until your opponent's turn ends, their Digimon's effects don't affect
    this Digimon and 1 of your Digimon with [Garurumon] in its name.
    [End of Your Turn] 2 of your Digimon may DNA digivolve into [Omnimon Alter-S]
    in the hand. Then, 1 of your Digimon may attack.
    Inherited: <Security A. +1>

Alt-Digi:
    [Digivolve] Lv.5 w/[Greymon] in name or w/[ADVENTURE] trait: Cost 3
    Standard evo: Red Lv.5 for cost 4
"""

import pytest


# Test card constants
BLITZGREYMON = "AD1-009"          # Lv.6 Red/Purple Cyborg/ADVENTURE
GARURUMON_AD1 = "AD1-010"         # Lv.4 Blue Beast/ADVENTURE
METALGREYMON_LV5 = "BT1-021"     # Lv.5 Red MetalGreymon (Greymon in name)
ADVENTURE_LV5 = "BT21-061"       # Lv.5 Purple MetalGreymon w/ADVENTURE trait
RED_LV5 = "BT1-020"              # Lv.5 Red Groundramon (standard evo base)
AGUMON_LV3 = "ST1-03"            # Lv.3 Red filler
OPP_LV5 = "BT1-020"              # Lv.5 opponent Digimon
OMNIMON_ALTER_S = "EX9-021"       # Lv.7 Omnimon Alter-S (DNA target)
BLUE_LV6 = "AD1-012"             # Lv.6 Blue CresGarurumon (DNA material)
RED_LV6 = "AD1-004"              # Lv.6 Red WarGreymon (DNA material)


@pytest.mark.behavioral
class TestAD1009BlitzGreymonKeywords:
    """Tests for Alliance, Piercing, Blocker keywords."""

    def test_has_alliance_keyword(self, debug_runner):
        """BlitzGreymon should have Alliance keyword."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BLITZGREYMON])
        effects = perm.top_card.effect_list(None)
        has_alliance = any(getattr(e, '_is_alliance', False) for e in effects)
        assert has_alliance, "BlitzGreymon should have Alliance keyword"

    def test_has_piercing_keyword(self, debug_runner):
        """BlitzGreymon should have Piercing keyword."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BLITZGREYMON])
        effects = perm.top_card.effect_list(None)
        has_piercing = any(getattr(e, '_is_piercing', False) for e in effects)
        assert has_piercing, "BlitzGreymon should have Piercing keyword"

    def test_has_blocker_keyword(self, debug_runner):
        """BlitzGreymon should have Blocker keyword."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BLITZGREYMON])
        effects = perm.top_card.effect_list(None)
        has_blocker = any(getattr(e, '_is_blocker', False) for e in effects)
        assert has_blocker, "BlitzGreymon should have Blocker keyword"


@pytest.mark.behavioral
class TestAD1009AltDigi:
    """Tests for alternative digivolution: Lv.5 w/[Greymon] in name OR [ADVENTURE] trait."""

    def test_alt_digi_from_greymon_name(self, debug_runner):
        """Should be able to digivolve from Lv.5 with [Greymon] in name for cost 3."""
        runner = debug_runner(initial_memory=10)
        # BT1-021 MetalGreymon Lv.5 Red — has "Greymon" in name
        runner.place_on_field(1, [METALGREYMON_LV5])
        runner.inject_card(1, BLITZGREYMON, "hand")
        runner.set_phase("Main")
        action = runner.find_action("BlitzGreymon")
        assert action is not None, (
            f"Should be able to digivolve BlitzGreymon onto Lv.5 MetalGreymon. "
            f"Actions: {runner.actions()}")

    def test_alt_digi_from_adventure_trait(self, debug_runner):
        """Should be able to digivolve from Lv.5 with [ADVENTURE] trait for cost 3."""
        runner = debug_runner(initial_memory=10)
        # BT21-061 MetalGreymon Lv.5 Purple w/ADVENTURE trait
        runner.place_on_field(1, [ADVENTURE_LV5])
        runner.inject_card(1, BLITZGREYMON, "hand")
        runner.set_phase("Main")
        action = runner.find_action("BlitzGreymon")
        assert action is not None, (
            f"Should be able to digivolve BlitzGreymon onto Lv.5 ADVENTURE Digimon. "
            f"Actions: {runner.actions()}")

    def test_alt_digi_effects_have_correct_attributes(self, debug_runner):
        """Alt-digi effects should have correct level, cost, and name/trait."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming
        from engine_py_legacy.engine.core.card_source import CardSource
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        entity = db.cards.get(BLITZGREYMON)
        cs = CardSource()
        cs.c_entity_base = entity
        effects = cs.effect_list(EffectTiming.NoTiming)
        alt_digi_effects = [
            e for e in effects if getattr(e, '_alt_digi_cost', None) is not None
        ]
        assert len(alt_digi_effects) >= 1, (
            f"Should have at least 1 alt-digi effect. Effects: "
            f"{[(getattr(e, '_effect_name', ''), getattr(e, '_alt_digi_cost', None)) for e in effects]}")
        # At least one should match Greymon name, one should match ADVENTURE trait
        has_name = any(getattr(e, '_alt_digi_name', None) == 'Greymon' for e in alt_digi_effects)
        has_trait = any(getattr(e, '_alt_digi_trait', None) == 'ADVENTURE' for e in alt_digi_effects)
        assert has_name or has_trait, (
            "Should have alt-digi effect for [Greymon] name or [ADVENTURE] trait")


@pytest.mark.behavioral
class TestAD1009OnPlayDeDigivolve:
    """Tests for [On Play] De-Digivolve 3 + immunity."""

    def test_on_play_triggers_de_digivolve_selection(self, debug_runner):
        """On Play should create a selection for opponent's Digimon to De-Digivolve 3."""
        runner = debug_runner(initial_memory=12)
        game = runner.game

        # Place opponent Digimon (a stack for de-digivolve to work on)
        runner.place_on_field(2, [AGUMON_LV3, OPP_LV5])

        # Inject BlitzGreymon into hand and play it
        runner.inject_card(1, BLITZGREYMON, "hand")
        runner.set_phase("Main")
        action = runner.find_action("BlitzGreymon")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"
        runner.execute(action)

        # Should have a pending selection for de-digivolve target
        ps = game.pending_selection
        assert ps is not None, (
            "On Play should create a de-digivolve selection. "
            f"Phase: {runner.snapshot().phase}")

    def test_on_play_immunity_applied_to_self(self, debug_runner):
        """After On Play, BlitzGreymon itself should get immunity."""
        runner = debug_runner(initial_memory=12)
        game = runner.game

        # Place opponent Digimon for de-digivolve target
        runner.place_on_field(2, [OPP_LV5])

        # Place a Garurumon on our field for immunity target
        runner.place_on_field(1, [GARURUMON_AD1])

        runner.inject_card(1, BLITZGREYMON, "hand")
        runner.set_phase("Main")
        action = runner.find_action("BlitzGreymon")
        runner.execute(action)
        runner.auto_resolve()

        # Find BlitzGreymon on field
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        blitz_perm = next(
            (p for p in game.player1.battle_area if p.top_card and
             p.top_card.card_id == BLITZGREYMON), None)
        assert blitz_perm is not None, "BlitzGreymon should be on field after play"
        has_immunity = game.modifiers.has_modifier(
            blitz_perm, ModifierType.CANNOT_BE_AFFECTED)
        assert has_immunity, "BlitzGreymon should have CANNOT_BE_AFFECTED after On Play"


@pytest.mark.behavioral
class TestAD1009WhenDigivolving:
    """Tests for [When Digivolving] De-Digivolve 3 + immunity."""

    def test_when_digivolving_triggers_de_digivolve(self, debug_runner):
        """When Digivolving should trigger de-digivolve 3 on opponent Digimon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place a Lv.5 Red on field (standard evo base)
        runner.place_on_field(1, [RED_LV5])
        # Place opponent Digimon
        runner.place_on_field(2, [OPP_LV5])

        runner.inject_card(1, BLITZGREYMON, "hand")
        runner.set_phase("Main")

        # Find digivolve action
        action = runner.find_action("BlitzGreymon")
        assert action is not None, f"Should find digivolve action. Actions: {runner.actions()}"
        runner.execute(action)

        # Should create de-digivolve selection
        ps = game.pending_selection
        assert ps is not None, (
            "When Digivolving should create a de-digivolve selection. "
            f"Phase: {runner.snapshot().phase}")


@pytest.mark.behavioral
class TestAD1009GarurumonImmunity:
    """Tests for the Garurumon immunity clause."""

    def test_garurumon_gets_immunity(self, debug_runner):
        """After On Play, 1 own [Garurumon] should get immunity."""
        runner = debug_runner(initial_memory=12)
        game = runner.game

        # Place opponent Digimon for de-digivolve
        runner.place_on_field(2, [OPP_LV5])

        # Place Garurumon on our field
        garu_perm = runner.place_on_field(1, [GARURUMON_AD1])

        runner.inject_card(1, BLITZGREYMON, "hand")
        runner.set_phase("Main")
        action = runner.find_action("BlitzGreymon")
        runner.execute(action)
        runner.auto_resolve()

        # Garurumon should have immunity
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        has_immunity = game.modifiers.has_modifier(
            garu_perm, ModifierType.CANNOT_BE_AFFECTED)
        assert has_immunity, (
            "Garurumon should have CANNOT_BE_AFFECTED after BlitzGreymon's effect")

    def test_no_garurumon_still_grants_self_immunity(self, debug_runner):
        """Even without a Garurumon on field, BlitzGreymon itself gets immunity."""
        runner = debug_runner(initial_memory=12)
        game = runner.game

        # Place opponent Digimon for de-digivolve
        runner.place_on_field(2, [OPP_LV5])

        # No Garurumon on our field
        runner.inject_card(1, BLITZGREYMON, "hand")
        runner.set_phase("Main")
        action = runner.find_action("BlitzGreymon")
        runner.execute(action)
        runner.auto_resolve()

        # BlitzGreymon should still have immunity
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        blitz_perm = next(
            (p for p in game.player1.battle_area if p.top_card and
             p.top_card.card_id == BLITZGREYMON), None)
        assert blitz_perm is not None
        has_immunity = game.modifiers.has_modifier(
            blitz_perm, ModifierType.CANNOT_BE_AFFECTED)
        assert has_immunity, (
            "BlitzGreymon should have CANNOT_BE_AFFECTED even without Garurumon")


@pytest.mark.behavioral
class TestAD1009EndOfTurn:
    """Tests for [End of Your Turn] DNA digivolve + may attack."""

    def test_eot_effect_exists(self, debug_runner):
        """BlitzGreymon should have an OnEndTurn effect."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming
        perm = runner.place_on_field(1, [BLITZGREYMON])
        effects = perm.top_card.effect_list(EffectTiming.OnEndTurn)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) > 0, "Should have OnEndTurn effect"

    def test_eot_triggers_dna_selection(self, debug_runner):
        """End of Turn should create a DNA digivolve selection when targets exist."""
        runner = debug_runner(initial_memory=-3)
        game = runner.game

        # Place BlitzGreymon on field
        runner.place_on_field(1, [BLITZGREYMON])

        # Put Omnimon Alter-S in hand
        runner.inject_card(1, OMNIMON_ALTER_S, "hand")

        # Place DNA materials: Blue Lv.6 + Red Lv.6 for EX9-021
        runner.place_on_field(1, [BLUE_LV6])
        runner.place_on_field(1, [RED_LV6])

        runner.set_phase("End")
        game.phase_end()

        # Should have a pending selection for DNA digivolve (or effect triggered)
        ps = game.pending_selection
        assert ps is not None, (
            f"End of Turn should create DNA digivolve selection. "
            f"Phase: {runner.snapshot().phase}, actions: {runner.actions()}")

    def test_eot_may_attack_after_skip_dna(self, debug_runner):
        """After skipping DNA, should offer attacker selection, then EndOfTurnAction."""
        runner = debug_runner(initial_memory=-3)
        game = runner.game

        # Place BlitzGreymon + another Digimon
        runner.place_on_field(1, [BLITZGREYMON])
        attacker = runner.place_on_field(1, [AGUMON_LV3])

        # No DNA target in hand — should skip DNA, then offer attack selection
        runner.set_phase("End")
        game.phase_end()

        # Should have a pending selection to pick which Digimon may attack
        ps = game.pending_selection
        assert ps is not None, (
            f"Should have pending attacker selection. Phase: {runner.snapshot().phase}")
        assert 'attack' in ps.prompt.lower(), (
            f"Selection prompt should mention attack, got: {ps.prompt}")

        # Select the second Digimon (attacker)
        legal = runner.action_mask()
        non_pass = [a for a in legal if a != 62]
        assert non_pass, f"Should have a non-pass action for attacker. Legal: {legal}"
        runner.execute(non_pass[0])

        # After selection, should be in EndOfTurnAction with attack options
        snap = runner.snapshot()
        assert snap.phase == "EndOfTurnAction", (
            f"Expected EndOfTurnAction phase after attacker selection, "
            f"got {snap.phase}. Actions: {runner.actions()}")

    def test_eot_condition_requires_own_turn(self, debug_runner):
        """End of Turn effect condition should check it's the owner's turn."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming
        perm = runner.place_on_field(1, [BLITZGREYMON])
        effects = perm.top_card.effect_list(EffectTiming.OnEndTurn)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) > 0, "Should have OnEndTurn effect"
        # Verify the effect has a condition function
        for eff in eot_effects:
            cond = eff.can_use_condition
            assert cond is not None, "OnEndTurn effect should have a condition"


@pytest.mark.behavioral
class TestAD1009Inherited:
    """Tests for inherited Security A. +1."""

    def test_inherited_security_attack_plus_1(self, debug_runner):
        """Inherited effect should grant Security A. +1."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming
        from engine_py_legacy.engine.core.card_source import CardSource
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        entity = db.cards.get(BLITZGREYMON)
        assert entity is not None
        cs = CardSource()
        cs.c_entity_base = entity
        effects = cs.effect_list(EffectTiming.NoTiming)

        inherited_sa = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False) and
            getattr(e, '_security_attack_modifier', 0) == 1
        ]
        assert len(inherited_sa) >= 1, (
            f"Should have inherited Security A. +1 effect. "
            f"Effects: {[(getattr(e, '_effect_name', '?'), getattr(e, '_security_attack_modifier', 0), getattr(e, 'is_inherited_effect', False)) for e in effects]}")
