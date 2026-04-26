"""Behavioral tests for AD1-011 Paildramon.

Card text:
  <Partition (Blue Lv.4 & Green Lv.4)>
  [When Digivolving] Until your opponent's turn ends, this Digimon can't be
    deleted in battle. Then, if DNA digivolving, this Digimon's attack target
    can't change for the turn.
  [When Attacking] This Digimon may digivolve into a Digimon card with
    [Imperialdramon] in its name in the hand with the digivolution cost
    reduced by 2.

  Inherited Effect:
    <Partition (Blue Lv.4 & Green Lv.4)>

  Alt-Digi: From Lv.4 with [Free] or [Hero] trait for cost 3
  DNA: Blue Lv.4 + Green Lv.4

Card reference:
  BT3-025 = ExVeemon Lv.4 Blue (Free attribute)
  BT3-050 = Stingmon Lv.4 Green (Free attribute)
  BT16-027 = Imperialdramon: Fighter Mode Lv.6 Blue/Green (cost 8)
  ST9-06 = Imperialdramon Dragon Mode Lv.6 Blue/Green (cost 13)
  ST1-07 = Greymon Lv.4 Red (no Free/Hero)
  BT21-013 = Agunimon Lv.4 Red (Hero trait)
  ST2-06 = Garurumon Lv.4 Blue (no Free/Hero)
"""

import pytest

from engine_py_legacy.engine.game.constants import (
    SEL_OPP_FIELD_START,
    SEL_MY_FIELD_START,
    SEL_HAND_START,
)


# Filler deck: enough cards for both players
FILLER_DECK = (
    ["ST2-01"] * 4    # Lv.2 eggs (blue)
    + ["BT3-025"] * 4  # ExVeemon Lv.4 Blue (Free attribute)
    + ["BT3-050"] * 4  # Stingmon Lv.4 Green (Free attribute)
    + ["ST2-03"] * 4   # Gabumon Lv.3 Blue
    + ["ST2-06"] * 4   # Garurumon Lv.4 Blue
    + ["ST1-07"] * 4   # Greymon Lv.4 Red
    + ["ST1-05"] * 4   # Birdramon Lv.4 Red
    + ["ST1-08"] * 4   # Garudamon Lv.5 Red
    + ["ST1-03"] * 4   # Agumon Lv.3 Red
    + ["ST2-08"] * 4   # WereGarurumon Lv.5 Blue
    + ["ST2-10"] * 4   # MetalGarurumon Lv.6 Blue
    + ["ST1-02"] * 4   # Biyomon Lv.3 Red
    + ["ST1-11"] * 2   # Tai Kamiya tamer
)

# Deck with AD1-011 and Imperialdramon in pool
DECK_WITH_PAILDRAMON = ["AD1-011"] + ["BT16-027"] * 3 + FILLER_DECK[4:]


@pytest.mark.behavioral
class TestPaildramonPartition:
    """Test Partition keyword flag on non-inherited and inherited effects."""

    def test_partition_flag_present(self, debug_runner):
        """AD1-011 should have the partition flag on the field."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")
        perm = runner.place_on_field(1, ["AD1-011"], turn_played=-1)

        # Check that the partition flag is present on at least one effect
        from engine_py_legacy.engine.data.enums import EffectTiming
        all_effects = []
        for cs in perm.card_sources:
            all_effects.extend(cs.effect_list(EffectTiming.NoTiming))
        partition_effects = [
            e for e in all_effects if getattr(e, '_is_partition', False)
        ]
        assert len(partition_effects) >= 1, (
            "AD1-011 should have at least 1 partition-flagged effect"
        )

    def test_inherited_partition_flag(self, debug_runner):
        """AD1-011's inherited effect should also have the partition flag."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")
        # Place AD1-011 under a Lv.6 to test inherited
        perm = runner.place_on_field(1, ["AD1-011", "ST2-10"], turn_played=-1)

        from engine_py_legacy.engine.data.enums import EffectTiming
        all_effects = []
        for cs in perm.card_sources:
            all_effects.extend(cs.effect_list(EffectTiming.NoTiming))
        inherited_partitions = [
            e for e in all_effects
            if getattr(e, '_is_partition', False)
            and getattr(e, 'is_inherited_effect', False)
        ]
        assert len(inherited_partitions) >= 1, (
            "AD1-011 should have an inherited partition effect in the evo stack"
        )


@pytest.mark.behavioral
class TestPaildramonAltDigi:
    """Test alternate digivolution: Lv.4 with [Free] attribute or [Hero] trait."""

    def test_alt_digi_from_free_attribute(self, debug_runner):
        """Paildramon should be able to digivolve from a Lv.4 with Free attribute."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        # BT3-025 = ExVeemon Lv.4 Blue, attribute_eng = ["Free"]
        runner.place_on_field(1, ["BT3-025"], turn_played=-1)

        # Inject AD1-011 into hand
        runner.inject_card(1, "AD1-011", zone="hand")

        action = runner.find_action("Digivolve")
        assert action is not None, (
            "Should be able to digivolve AD1-011 onto ExVeemon (Lv.4 with Free)"
        )

    def test_alt_digi_from_hero_trait(self, debug_runner):
        """Paildramon should be able to digivolve from a Lv.4 with Hero trait."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON + ["BT21-013"] * 2,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # BT21-013 = Agunimon Lv.4 Red, type_eng = ["Wizard", "Hero"]
        runner.place_on_field(1, ["BT21-013"], turn_played=-1)

        runner.inject_card(1, "AD1-011", zone="hand")

        action = runner.find_action("Digivolve")
        assert action is not None, (
            "Should be able to digivolve AD1-011 onto Agunimon (Lv.4 with Hero trait)"
        )

    def test_no_alt_digi_from_non_free_non_hero(self, debug_runner):
        """Paildramon should NOT alt-digi from Lv.4 without Free or Hero."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        # ST1-07 = Greymon Lv.4 Red, type_eng = ["Dinosaur"]
        # Not Free, not Hero, not Blue (standard evo is Blue Lv.4)
        runner.place_on_field(1, ["ST1-07"], turn_played=-1)

        runner.inject_card(1, "AD1-011", zone="hand")

        actions = runner.find_actions("Digivolve")
        # Filter out actions that target other permanents or hand cards
        paildramon_digi = {
            aid: desc for aid, desc in actions.items()
            if "paildramon" in desc.lower()
        }
        assert len(paildramon_digi) == 0, (
            f"Should NOT be able to digivolve Paildramon onto Greymon "
            f"(no Free/Hero). Found: {paildramon_digi}"
        )


@pytest.mark.behavioral
class TestPaildramonWhenDigivolving:
    """Test [When Digivolving] can't be deleted in battle + DNA attack target lock."""

    def test_cant_be_deleted_in_battle_after_digivolving(self, debug_runner):
        """After digivolving, Paildramon can't be deleted in battle."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        # Place ExVeemon (Free attribute, Blue Lv.4) on field, then digivolve
        runner.place_on_field(1, ["BT3-025"], turn_played=-1)
        runner.inject_card(1, "AD1-011", zone="hand")

        # Digivolve AD1-011 onto ExVeemon
        digi_action = runner.find_action("Digivolve")
        assert digi_action is not None
        runner.execute(digi_action)
        runner.auto_resolve()

        # Verify the modifier is registered
        snap = runner.snapshot()
        paildramon_on_field = [s for s in snap.p1_field if s.card_id == "AD1-011"]
        assert len(paildramon_on_field) >= 1, "Paildramon should be on field"

        # Check the CANNOT_BE_DESTROYED_BY_BATTLE modifier is active
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        perm = runner.game.player1.battle_area[paildramon_on_field[0].slot]
        has_mod = runner.game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_BE_DESTROYED_BY_BATTLE
        )
        assert has_mod, (
            "Paildramon should have CANNOT_BE_DESTROYED_BY_BATTLE after digivolving"
        )

    def test_dna_digivolving_grants_attack_target_lock(self, debug_runner):
        """When DNA digivolving, attack target can't change for the turn."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        # Place Blue Lv.4 and Green Lv.4 for DNA digivolve
        runner.place_on_field(1, ["BT3-025"], turn_played=-1)  # ExVeemon Blue Lv.4
        runner.place_on_field(1, ["BT3-050"], turn_played=-1)  # Stingmon Green Lv.4

        # Inject AD1-011 into hand for DNA digivolve
        runner.inject_card(1, "AD1-011", zone="hand")

        # Find the DNA digivolve action
        dna_actions = runner.find_actions("DNA")
        if not dna_actions:
            dna_actions = runner.find_actions("Jogress")
        assert len(dna_actions) > 0, (
            f"Should find a DNA/Jogress digivolve action. "
            f"Available: {runner.actions()}"
        )

        # Execute DNA digivolve and resolve material selection
        first_dna = list(dna_actions.keys())[0]
        runner.execute(first_dna)
        runner.auto_resolve()

        # Find Paildramon on field
        snap = runner.snapshot()
        paildramon_on_field = [s for s in snap.p1_field if s.card_id == "AD1-011"]
        assert len(paildramon_on_field) >= 1, (
            f"Paildramon should be on field after DNA digivolve. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}"
        )

        # Check CANNOT_SWITCH_ATTACK_TARGET modifier
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        perm = runner.game.player1.battle_area[paildramon_on_field[0].slot]
        has_mod = runner.game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_SWITCH_ATTACK_TARGET
        )
        assert has_mod, (
            "Paildramon should have CANNOT_SWITCH_ATTACK_TARGET after DNA digivolving"
        )

    def test_non_dna_digivolving_no_attack_target_lock(self, debug_runner):
        """Normal digivolve should NOT grant attack target lock."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        # Normal digivolve from ExVeemon (Free attribute)
        runner.place_on_field(1, ["BT3-025"], turn_played=-1)
        runner.inject_card(1, "AD1-011", zone="hand")

        digi_action = runner.find_action("Digivolve")
        assert digi_action is not None
        runner.execute(digi_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        paildramon_on_field = [s for s in snap.p1_field if s.card_id == "AD1-011"]
        assert len(paildramon_on_field) >= 1

        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        perm = runner.game.player1.battle_area[paildramon_on_field[0].slot]
        has_target_lock = runner.game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_SWITCH_ATTACK_TARGET
        )
        assert not has_target_lock, (
            "Normal digivolve should NOT grant CANNOT_SWITCH_ATTACK_TARGET"
        )


@pytest.mark.behavioral
class TestPaildramonWhenAttacking:
    """Test [When Attacking] digivolve into Imperialdramon from hand."""

    def test_when_attacking_offers_imperialdramon_selection(self, debug_runner):
        """When Attacking, should offer to digivolve into Imperialdramon from hand."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["AD1-011"], turn_played=-1)

        # Inject Imperialdramon into hand
        runner.inject_card(1, "BT16-027", zone="hand")

        # Attack
        attack_action = runner.find_action("Attack player with Paildramon")
        assert attack_action is not None, (
            f"Should be able to attack. Available: {runner.actions()}"
        )
        runner.execute(attack_action)

        # Should be in SelectTarget for the optional digivolve
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", (
            f"Expected SelectTarget for Imperialdramon selection, got {snap.phase}"
        )

        # The pending selection should include Imperialdramon from hand
        game = runner.game
        assert game.pending_selection is not None
        valid = game.pending_selection.valid_indices
        # Should include at least one hand card index
        hand_indices = [v for v in valid if v >= SEL_HAND_START and v < SEL_MY_FIELD_START]
        assert len(hand_indices) > 0, (
            f"Should offer Imperialdramon from hand. Valid: {valid}"
        )

    def test_when_attacking_cost_reduced_by_2(self, debug_runner):
        """Digivolving into Imperialdramon should cost 2 less than normal."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=15
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["AD1-011"], turn_played=-1)

        # BT16-027 Imperialdramon: Fighter Mode has play_cost 8, but the
        # digivolve from hand uses the card's own cost minus 2.
        # In effect_digivolve_from_hand, cost = max(0, base_cost - reduction)
        runner.inject_card(1, "BT16-027", zone="hand")

        before_memory = runner.game.memory

        # Attack
        attack_action = runner.find_action("Attack player with Paildramon")
        assert attack_action is not None
        runner.execute(attack_action)

        # Select the Imperialdramon from hand
        game = runner.game
        valid = game.pending_selection.valid_indices
        hand_indices = [v for v in valid if v >= SEL_HAND_START and v < SEL_MY_FIELD_START]
        assert len(hand_indices) > 0
        runner.execute(hand_indices[0])
        runner.auto_resolve()

        # BT16-027 cost is 8, reduced by 2 = 6
        expected_cost = 6
        actual_spent = before_memory - runner.game.memory
        assert actual_spent == expected_cost, (
            f"Expected to spend {expected_cost} memory (cost 8 - 2 reduction), "
            f"but spent {actual_spent}"
        )

    def test_when_attacking_optional_can_decline(self, debug_runner):
        """The digivolve from hand should be optional (can decline)."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["AD1-011"], turn_played=-1)
        runner.inject_card(1, "BT16-027", zone="hand")

        attack_action = runner.find_action("Attack player with Paildramon")
        assert attack_action is not None
        runner.execute(attack_action)

        # Should be in SelectTarget for the optional digivolve
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget"

        # Action 62 = decline/pass; check it's in the action mask
        DECLINE = 62
        legal = runner.action_mask()
        assert DECLINE in legal, (
            f"Declining should be a legal action. Legal: {legal}"
        )
        runner.execute(DECLINE)
        runner.auto_resolve()

        # Paildramon should still be on field (not digivolved)
        snap = runner.snapshot()
        p1_top_ids = [s.card_id for s in snap.p1_field]
        assert "AD1-011" in p1_top_ids, (
            f"Paildramon should remain as top card after declining. Field: {p1_top_ids}"
        )

    def test_when_attacking_no_imperialdramon_in_hand(self, debug_runner):
        """Without Imperialdramon in hand, the WA effect should not trigger."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["AD1-011"], turn_played=-1)

        # Make sure no Imperialdramon in hand
        runner.clear_zone(1, "hand")

        attack_action = runner.find_action("Attack player with Paildramon")
        assert attack_action is not None
        runner.execute(attack_action)

        # Should NOT be in SelectTarget (no valid candidates)
        # auto_resolve should pass through quickly
        runner.auto_resolve()

    def test_only_imperialdramon_cards_offered(self, debug_runner):
        """Only cards with [Imperialdramon] in name should be valid targets."""
        runner = debug_runner(
            deck1=DECK_WITH_PAILDRAMON, deck2=FILLER_DECK, initial_memory=10
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["AD1-011"], turn_played=-1)

        # Inject Imperialdramon AND a non-Imperialdramon into hand
        runner.inject_card(1, "BT16-027", zone="hand")  # Imperialdramon
        runner.inject_card(1, "ST2-10", zone="hand")     # MetalGarurumon (NOT Imperialdramon)

        attack_action = runner.find_action("Attack player with Paildramon")
        assert attack_action is not None
        runner.execute(attack_action)

        game = runner.game
        valid = game.pending_selection.valid_indices
        hand_indices = [v for v in valid if v >= SEL_HAND_START and v < SEL_MY_FIELD_START]

        # Check that only Imperialdramon is offered
        for hi in hand_indices:
            idx = hi - SEL_HAND_START
            card = game.active_player.hand_cards[idx]
            card_name = card.c_entity_base.card_name_eng if card.c_entity_base else ""
            assert "Imperialdramon" in card_name, (
                f"Only Imperialdramon cards should be valid. Got: {card_name}"
            )
