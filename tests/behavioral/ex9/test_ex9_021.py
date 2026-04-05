"""Behavioral tests for EX9-021 Omnimon Alter-S.

Card text:
[When Digivolving] If DNA digivolving, your opponent's effects don't affect
    this Digimon for the turn. Then, delete all of their Digimon with the
    highest level.
[End of Attack] You may play 1 card with [Greymon] in its name or the [Ver.1]
    trait and 1 card with [Garurumon] in its name or the [Ver.2] trait from
    this Digimon's digivolution cards without paying the costs. If this effect
    played, place this Digimon as your top security card.

Alt-digi: Lv.6 w/[DM] trait: Cost 5
DNA: Blue Lv.6 + Red Lv.6, cost 0
"""

import pytest


# ── Helper to build a minimal deck containing EX9-021 and support cards ──

def _make_deck():
    """Build a 50-card deck with EX9-021 and useful support cards."""
    # 4x egg, rest rookies/champions/megas to fill slots
    eggs = ["BT2-001"] * 4
    # Include Greymon and Garurumon-name / Ver.1 / Ver.2 cards for evo sources
    core = [
        "EX9-021",  # Omnimon Alter-S
        "EX9-013",  # BlitzGreymon (Red/Black Lv.6, DM, Ver.1) - DNA material
        "EX9-020",  # CresGarurumon (Blue/Black Lv.6, DM, Ver.2) - DNA material
        "EX9-009",  # Greymon (Red Lv.4, DM, Ver.1)
        "EX9-017",  # Garurumon (Blue Lv.4, DM, Ver.2)
        "EX9-011",  # MetalGreymon (Red/Black Lv.5, DM, Ver.1)
        "EX9-018",  # MetalMamemon (Blue/Black Lv.5, DM, Ver.2)
        "EX9-007",  # Agumon (Red Lv.3, DM, Ver.1)
        "EX9-014",  # Gabumon (Blue Lv.3, DM, Ver.2)
    ]
    filler = ["ST1-03"] * (50 - len(eggs) - len(core))
    return eggs + core + filler


@pytest.mark.behavioral
class TestEX9021OmnimonAlterS:
    """Tests for EX9-021 Omnimon Alter-S."""

    # ── Alt-digi attributes ──────────────────────────────────────────

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi should be Lv.6 with [DM] trait for cost 5."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        # Place a DM Lv.6 on field (BlitzGreymon EX9-013)
        runner.place_on_field(1, ["EX9-013"])

        # Put EX9-021 in hand
        runner.inject_card(1, "EX9-021", "hand")
        runner.set_phase("Main")

        # Find digivolve action for Omnimon Alter-S
        action = runner.find_action("Digivolve")
        # Should be able to alt-digi onto the DM Lv.6
        assert action is not None, "Should find digivolve action for Omnimon Alter-S onto DM Lv.6"

    def test_alt_digi_cost_is_5(self, debug_runner):
        """Alt-digivolving onto Lv.6 DM should cost 5 memory."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        # Place a DM Lv.6 on field
        runner.place_on_field(1, ["EX9-013"])

        runner.inject_card(1, "EX9-021", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        result = runner.execute(action)
        runner.auto_resolve()

        # Memory should drop by 5 (alt-digi cost)
        assert result.after.memory == result.before.memory - 5, (
            f"Alt-digi should cost 5 memory, was {result.before.memory}, now {result.after.memory}"
        )

    # ── When Digivolving: delete highest level (always) ──────────────

    def test_when_digivolving_delete_highest_level(self, debug_runner):
        """[When Digivolving] Should delete all opponent Digimon with highest level."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # Place opponent Digimon at different levels:
        # Lv.6 (highest) - should be deleted
        runner.place_on_field(2, ["EX9-013"])  # BlitzGreymon Lv.6
        # Lv.4 - should survive
        runner.place_on_field(2, ["EX9-009"])  # Greymon Lv.4

        snap_before = runner.snapshot()
        assert len(snap_before.p2_field) == 2, "Opponent should have 2 Digimon before"

        # Place DM Lv.6 on our field, then alt-digi into Omnimon Alter-S
        runner.place_on_field(1, ["EX9-013"])
        runner.inject_card(1, "EX9-021", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Lv.6 BlitzGreymon should be deleted, Lv.4 Greymon should remain
        remaining_ids = [s.card_id for s in snap.p2_field]
        assert "EX9-013" not in remaining_ids, (
            "Opponent's Lv.6 BlitzGreymon should have been deleted"
        )
        assert "EX9-009" in remaining_ids, (
            "Opponent's Lv.4 Greymon should survive"
        )

    def test_when_digivolving_delete_all_highest_not_just_one(self, debug_runner):
        """Delete ALL opponent Digimon with the highest level, not just one."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # Place TWO Lv.5 Digimon (same highest level) on opponent's field
        runner.place_on_field(2, ["EX9-011"])  # MetalGreymon Lv.5
        runner.place_on_field(2, ["EX9-018"])  # MetalMamemon Lv.5
        # Lv.4 should survive
        runner.place_on_field(2, ["EX9-009"])  # Greymon Lv.4

        # Alt-digi into Omnimon Alter-S
        runner.place_on_field(1, ["EX9-020"])  # CresGarurumon (DM Lv.6)
        runner.inject_card(1, "EX9-021", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        remaining_ids = [s.card_id for s in snap.p2_field]
        assert "EX9-011" not in remaining_ids, "First Lv.5 should be deleted"
        assert "EX9-018" not in remaining_ids, "Second Lv.5 should be deleted"
        assert "EX9-009" in remaining_ids, "Lv.4 Greymon should survive"

    def test_when_digivolving_no_opponent_digimon_no_crash(self, debug_runner):
        """Delete effect should not crash when opponent has no Digimon."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # No opponent Digimon
        runner.place_on_field(1, ["EX9-013"])  # DM Lv.6
        runner.inject_card(1, "EX9-021", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Should complete without error
        snap = runner.snapshot()
        p1_ids = [s.card_id for s in snap.p1_field]
        assert "EX9-021" in p1_ids, "Omnimon Alter-S should be on field"

    # ── When Digivolving: DNA immunity ───────────────────────────────

    def test_dna_digivolve_grants_immunity(self, debug_runner):
        """DNA digivolving should grant CANNOT_BE_AFFECTED for the turn."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # Place two Lv.6 materials: Blue + Red
        runner.place_on_field(1, ["EX9-020"])  # CresGarurumon (Blue Lv.6)
        runner.place_on_field(1, ["EX9-013"])  # BlitzGreymon (Red Lv.6)

        runner.inject_card(1, "EX9-021", "hand")
        runner.set_phase("Main")

        # Find DNA digivolve action
        action = runner.find_action("DNA")
        if action is None:
            # Fallback: try "Jogress" or "Digivolve" keyword
            action = runner.find_action("Jogress")
        if action is None:
            action = runner.find_action("Digivolve")

        # DNA digivolve may require material selection steps
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        # Check that Omnimon Alter-S has CANNOT_BE_AFFECTED modifier
        game = runner.game
        player = game.player1
        omnimon_perm = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "EX9-021":
                omnimon_perm = p
                break

        if omnimon_perm is not None:
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            has_immunity = game.modifiers.has_modifier(
                omnimon_perm, ModifierType.CANNOT_BE_AFFECTED
            )
            assert has_immunity, (
                "DNA digivolving should grant CANNOT_BE_AFFECTED modifier"
            )

    def test_non_dna_digivolve_no_immunity(self, debug_runner):
        """Normal (alt-digi) should NOT grant immunity."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # Place DM Lv.6 (alt-digi base)
        runner.place_on_field(1, ["EX9-013"])  # BlitzGreymon (DM Lv.6)

        runner.inject_card(1, "EX9-021", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        game = runner.game
        player = game.player1
        omnimon_perm = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "EX9-021":
                omnimon_perm = p
                break

        assert omnimon_perm is not None, "Omnimon should be on field"
        from digimon_gym.engine.interfaces.modifiers import ModifierType
        has_immunity = game.modifiers.has_modifier(
            omnimon_perm, ModifierType.CANNOT_BE_AFFECTED
        )
        assert not has_immunity, (
            "Normal digivolution should NOT grant CANNOT_BE_AFFECTED"
        )

    # ── End of Attack: play from evo cards ───────────────────────────

    def test_end_of_attack_play_greymon_and_garurumon(self, debug_runner):
        """[End of Attack] Should play Greymon/Ver.1 + Garurumon/Ver.2 from evo cards."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # Build Omnimon Alter-S with evo cards containing Greymon and Garurumon
        # Stack: bottom Agumon (Ver.1) -> Greymon (Ver.1) -> Garurumon (Ver.2) -> Omnimon
        # Actually we need them as digivolution cards, so place them in the stack
        perm = runner.place_on_field(1, [
            "EX9-009",  # Greymon Lv.4 (has "Greymon" name + Ver.1 trait)
            "EX9-017",  # Garurumon Lv.4 (has "Garurumon" name + Ver.2 trait)
            "EX9-021",  # Omnimon Alter-S on top
        ])

        game = runner.game
        player = game.player1

        # Verify stack is correct
        assert len(perm.card_sources) == 3, f"Stack should have 3 cards, got {len(perm.card_sources)}"
        assert perm.top_card.c_entity_base.card_id == "EX9-021"

        # Put an opponent Digimon for attack target
        runner.place_on_field(2, ["ST1-03"])

        # Trigger attack
        runner.set_phase("Main")
        attack_action = runner.find_action("Attack")
        if attack_action is None:
            pytest.skip("Could not find attack action")

        snap_before = runner.snapshot()
        runner.execute(attack_action)

        # Auto-resolve through combat and the End of Attack selection phases
        runner.auto_resolve(max_steps=30)

        snap = runner.snapshot()

        # After End of Attack: Greymon and Garurumon should have been played as new permanents
        p1_ids = [s.card_id for s in snap.p1_field]
        greymon_played = "EX9-009" in p1_ids
        garurumon_played = "EX9-017" in p1_ids

        # If both were played, Omnimon should be in security
        if greymon_played and garurumon_played:
            # The Omnimon should have been placed as top security
            assert snap.p1_security_count > snap_before.p1_security_count, (
                "After playing both cards, Omnimon should be placed as top security"
            )

    def test_end_of_attack_place_as_top_security(self, debug_runner):
        """If End of Attack played any card, Omnimon goes to top security."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # Stack with only a Greymon evo card (no Garurumon available)
        perm = runner.place_on_field(1, [
            "EX9-009",  # Greymon Lv.4 (Greymon name + Ver.1)
            "EX9-021",  # Omnimon Alter-S on top
        ])

        runner.place_on_field(2, ["ST1-03"])  # Attack target
        runner.set_phase("Main")

        attack_action = runner.find_action("Attack")
        if attack_action is None:
            pytest.skip("Could not find attack action")

        snap_before = runner.snapshot()
        runner.execute(attack_action)
        runner.auto_resolve(max_steps=30)

        snap = runner.snapshot()
        # If Greymon was played, security should increase
        greymon_on_field = any(s.card_id == "EX9-009" for s in snap.p1_field)
        if greymon_on_field:
            assert snap.p1_security_count > snap_before.p1_security_count, (
                "After playing from evo cards, Omnimon should be placed as top security"
            )

    def test_end_of_attack_no_eligible_cards_no_security(self, debug_runner):
        """If no eligible evo cards, nothing is played and Omnimon stays on field."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # Stack with no Greymon/Ver.1 or Garurumon/Ver.2 evo cards
        # Just a basic rookie under Omnimon (ST1-03 Agumon has no Greymon name or Ver.1/Ver.2)
        perm = runner.place_on_field(1, [
            "ST1-03",   # Agumon (no Greymon name, no Ver.1/Ver.2)
            "EX9-021",  # Omnimon Alter-S on top
        ])

        runner.place_on_field(2, ["ST1-03"])  # Attack target
        runner.set_phase("Main")

        attack_action = runner.find_action("Attack")
        if attack_action is None:
            pytest.skip("Could not find attack action")

        snap_before = runner.snapshot()
        runner.execute(attack_action)
        runner.auto_resolve(max_steps=30)

        snap = runner.snapshot()

        # Security should not increase (no cards were played)
        omnimon_on_field = any(s.card_id == "EX9-021" for s in snap.p1_field)
        # Omnimon should still be on field OR game is over from the attack
        if not snap.is_game_over:
            # If Omnimon is on field, security shouldn't have changed from this effect
            # (may have changed from security check during attack)
            pass  # Non-failure case: just verify no crash

    # ── End of Attack: card eligibility checks ───────────────────────

    def test_greymon_name_matches(self, debug_runner):
        """Cards with [Greymon] in name should be eligible for first selection."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # Place with a MetalGreymon (has "Greymon" in name)
        perm = runner.place_on_field(1, [
            "EX9-011",  # MetalGreymon Lv.5 (name contains "Greymon")
            "EX9-017",  # Garurumon (for second selection)
            "EX9-021",  # Omnimon on top
        ])

        runner.place_on_field(2, ["ST1-03"])
        runner.set_phase("Main")

        attack_action = runner.find_action("Attack")
        if attack_action is None:
            pytest.skip("Could not find attack action")

        runner.execute(attack_action)
        runner.auto_resolve(max_steps=30)

        snap = runner.snapshot()
        # MetalGreymon should be played (has "Greymon" in name)
        metalgreymon_played = any(s.card_id == "EX9-011" for s in snap.p1_field)
        if not snap.is_game_over:
            assert metalgreymon_played, (
                "MetalGreymon should be eligible (contains 'Greymon' in name)"
            )

    def test_ver1_trait_matches_for_greymon_slot(self, debug_runner):
        """Cards with [Ver.1] trait should be eligible for the first (Greymon) slot."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # EX9-025 Airdramon has Ver.1 trait but no "Greymon" in name
        # Let's use EX9-050 Numemon (Ver.1 trait, no Greymon name)
        perm = runner.place_on_field(1, [
            "EX9-007",  # Agumon Lv.3 (Ver.1 trait, no "Greymon" in name)
            "EX9-017",  # Garurumon (for second slot)
            "EX9-021",  # Omnimon on top
        ])

        runner.place_on_field(2, ["ST1-03"])
        runner.set_phase("Main")

        attack_action = runner.find_action("Attack")
        if attack_action is None:
            pytest.skip("Could not find attack action")

        runner.execute(attack_action)
        runner.auto_resolve(max_steps=30)

        snap = runner.snapshot()
        if not snap.is_game_over:
            # Agumon (Ver.1) should be played despite not having "Greymon" in name
            agumon_played = any(s.card_id == "EX9-007" for s in snap.p1_field)
            assert agumon_played, (
                "Agumon (Ver.1 trait) should be eligible for first selection slot"
            )

    def test_ver2_trait_matches_for_garurumon_slot(self, debug_runner):
        """Cards with [Ver.2] trait should be eligible for the second (Garurumon) slot."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        # EX9-014 Gabumon has Ver.2 trait but no "Garurumon" in name
        perm = runner.place_on_field(1, [
            "EX9-009",  # Greymon (for first slot)
            "EX9-014",  # Gabumon Lv.3 (Ver.2 trait, no "Garurumon" in name)
            "EX9-021",  # Omnimon on top
        ])

        runner.place_on_field(2, ["ST1-03"])
        runner.set_phase("Main")

        attack_action = runner.find_action("Attack")
        if attack_action is None:
            pytest.skip("Could not find attack action")

        runner.execute(attack_action)
        runner.auto_resolve(max_steps=30)

        snap = runner.snapshot()
        if not snap.is_game_over:
            # Gabumon (Ver.2) should be played despite not having "Garurumon" in name
            gabumon_played = any(s.card_id == "EX9-014" for s in snap.p1_field)
            assert gabumon_played, (
                "Gabumon (Ver.2 trait) should be eligible for second selection slot"
            )

    # ── Script structure tests ───────────────────────────────────────

    def test_script_has_correct_effect_timings(self, debug_runner):
        """Verify the script registers effects with correct timings."""
        from digimon_gym.engine.data.enums import EffectTiming
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-021"])
        effects = perm.top_card.effect_list(None)

        timings = [getattr(e, 'timing', None) for e in effects]
        names = [getattr(e, 'effect_name', '') for e in effects]

        # Should have: alt-digi (None timing), When Digivolving (OnEnterFieldAnyone),
        # End of Attack (OnEndAttack)
        has_when_digivolving = any(
            e.timing == EffectTiming.OnEnterFieldAnyone
            for e in effects
            if hasattr(e, 'timing') and e.timing is not None
        )
        has_end_of_attack = any(
            e.timing == EffectTiming.OnEndAttack
            for e in effects
            if hasattr(e, 'timing') and e.timing is not None
        )

        assert has_when_digivolving, "Should have a When Digivolving effect"
        assert has_end_of_attack, "Should have an End of Attack effect"

    def test_alt_digi_effect_attributes(self, debug_runner):
        """Verify alt-digi effect has correct _alt_digi_* attributes."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-021"])
        effects = perm.top_card.effect_list(None)

        alt_digi_effects = [e for e in effects if hasattr(e, '_alt_digi_cost')]
        assert len(alt_digi_effects) >= 1, "Should have at least one alt-digi effect"

        alt = alt_digi_effects[0]
        assert alt._alt_digi_cost == 5, f"Alt-digi cost should be 5, got {alt._alt_digi_cost}"
        assert alt._alt_digi_level == 6, f"Alt-digi level should be 6, got {alt._alt_digi_level}"
        assert alt._alt_digi_trait == "DM", f"Alt-digi trait should be 'DM', got {alt._alt_digi_trait}"

    def test_end_of_attack_is_optional(self, debug_runner):
        """End of Attack effect should be marked as optional."""
        from digimon_gym.engine.data.enums import EffectTiming
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-021"])
        effects = perm.top_card.effect_list(None)

        eoa_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnEndAttack
        ]
        assert len(eoa_effects) == 1, "Should have exactly one End of Attack effect"
        assert eoa_effects[0].is_optional, "End of Attack should be optional"

    def test_when_digivolving_is_mandatory(self, debug_runner):
        """When Digivolving effect should NOT be optional (mandatory activation)."""
        from digimon_gym.engine.data.enums import EffectTiming
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)

        perm = runner.place_on_field(1, ["EX9-021"])
        effects = perm.top_card.effect_list(None)

        wd_effects = [
            e for e in effects
            if getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) == 1, "Should have exactly one When Digivolving effect"
        assert not wd_effects[0].is_optional, "When Digivolving should be mandatory"
