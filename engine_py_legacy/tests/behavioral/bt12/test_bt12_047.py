"""Behavioral tests for BT12-047 Wormmon.

Card text:
  Lv.3 | Green | DP:1000 | Cost:3
  [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
  [Imperialdramon] in its name or [Free] trait and 1 Tamer card with
  [Ken Ichijoji] in its name among them to your hand. Place the rest at
  the bottom of your deck in any order.

  Inherited: [End of Your Turn] This Digimon and any of your other Digimon
  may DNA digivolve into a Digimon card in the hand.

Key faithfulness points tested:
  - On Play reveals exactly 3 cards from top of deck.
  - Pass 1: add 1 Digimon with [Imperialdramon] in name OR [Free] trait.
  - Pass 2: add 1 Tamer with [Ken Ichijoji] in name.
  - Remaining cards placed at deck bottom.
  - [Free] trait lives in attribute_eng, not card_traits (type_eng).
  - Inherited: end-of-turn DNA digivolve from hand (engine gap #12 resolved).
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming

# Card IDs used in tests
BT12_047 = "BT12-047"              # Card under test (Wormmon, Lv.3 Green)
IMPERIALDRAMON_VIRUS = "BT20-020"  # Imperialdramon: Fighter Mode (Virus, NOT Free)
VEEMON_FREE = "BT2-021"            # Veemon Lv.3 (Free attribute, no Imperialdramon name)
KEN_ICHIJOJI = "BT3-094"           # Ken Ichijoji (Green Tamer)
AGUMON_FILLER = "ST1-03"           # Agumon Lv.3 (Vaccine, not Imperialdramon, not Free)
PAILDRAMON_DNA = "BT12-028"        # Paildramon Lv.5 (DNA: Blue Lv.4 + Green Lv.4)
BLUE_LV4 = "BT1-032"              # Blue Lv.4 Digimon (DNA material)
GREEN_LV4 = "BT1-069"             # Green Lv.4 Digimon (DNA material)


@pytest.mark.behavioral
class TestBT12047OnPlay:
    """[On Play] Reveal top 3, add Imperialdramon/Free Digimon + Ken Ichijoji Tamer."""

    def test_on_play_adds_imperialdramon_by_name(self, debug_runner):
        """Pass 1 should match a Digimon with [Imperialdramon] in its name (even non-Free)."""
        runner = debug_runner(initial_memory=10)

        # Clear deck and inject known cards on top
        runner.clear_zone(1, "library")
        runner.inject_card(1, IMPERIALDRAMON_VIRUS, "library_top")  # pos 0 (Imperialdramon, Virus)
        runner.inject_card(1, AGUMON_FILLER, "library_top")         # pos 1
        runner.inject_card(1, AGUMON_FILLER, "library_top")         # pos 2 (top)
        # Pad library so game doesn't crash
        for _ in range(10):
            runner.inject_card(1, AGUMON_FILLER, "library_bottom")

        runner.inject_card(1, BT12_047, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Wormmon")
        assert action is not None, f"Should find play action for Wormmon. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert IMPERIALDRAMON_VIRUS in snap.p1_hand, (
            f"Imperialdramon (Virus) should be added to hand by name match. "
            f"Hand: {snap.p1_hand}"
        )

    def test_on_play_adds_free_trait_digimon(self, debug_runner):
        """Pass 1 should match a Digimon with [Free] trait (attribute_eng, not type_eng).

        This tests the Free trait bug fix: Free is in attribute_eng, not card_traits.
        BT2-021 Veemon has attribute_eng=['Free'] but type_eng=['Mini Dragon'].
        """
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(1, "library")
        runner.inject_card(1, VEEMON_FREE, "library_top")     # pos 0 (Free, no Imperialdramon)
        runner.inject_card(1, AGUMON_FILLER, "library_top")   # pos 1
        runner.inject_card(1, AGUMON_FILLER, "library_top")   # pos 2 (top)
        for _ in range(10):
            runner.inject_card(1, AGUMON_FILLER, "library_bottom")

        runner.inject_card(1, BT12_047, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Wormmon")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert VEEMON_FREE in snap.p1_hand, (
            f"Veemon (Free attribute) should be added to hand by Free trait match. "
            f"Hand: {snap.p1_hand}. "
            f"Free is in attribute_eng, not card_traits/type_eng."
        )

    def test_on_play_adds_ken_ichijoji_tamer(self, debug_runner):
        """Pass 2 should match a Tamer with [Ken Ichijoji] in its name."""
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(1, "library")
        runner.inject_card(1, KEN_ICHIJOJI, "library_top")    # pos 0 (Ken Ichijoji Tamer)
        runner.inject_card(1, AGUMON_FILLER, "library_top")   # pos 1
        runner.inject_card(1, AGUMON_FILLER, "library_top")   # pos 2 (top)
        for _ in range(10):
            runner.inject_card(1, AGUMON_FILLER, "library_bottom")

        runner.inject_card(1, BT12_047, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Wormmon")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert KEN_ICHIJOJI in snap.p1_hand, (
            f"Ken Ichijoji should be added to hand by name match. "
            f"Hand: {snap.p1_hand}"
        )

    def test_on_play_both_passes(self, debug_runner):
        """Both passes should work together: add Imperialdramon + Ken Ichijoji."""
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(1, "library")
        runner.inject_card(1, KEN_ICHIJOJI, "library_top")             # pos 0
        runner.inject_card(1, IMPERIALDRAMON_VIRUS, "library_top")     # pos 1
        runner.inject_card(1, AGUMON_FILLER, "library_top")            # pos 2 (top)
        for _ in range(10):
            runner.inject_card(1, AGUMON_FILLER, "library_bottom")

        runner.inject_card(1, BT12_047, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Wormmon")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert IMPERIALDRAMON_VIRUS in snap.p1_hand, (
            f"Imperialdramon should be in hand. Hand: {snap.p1_hand}"
        )
        assert KEN_ICHIJOJI in snap.p1_hand, (
            f"Ken Ichijoji should be in hand. Hand: {snap.p1_hand}"
        )

    def test_on_play_no_matching_cards(self, debug_runner):
        """If no matching cards in top 3, all go to deck bottom, hand unchanged."""
        runner = debug_runner(initial_memory=10)

        runner.clear_zone(1, "library")
        for _ in range(13):
            runner.inject_card(1, AGUMON_FILLER, "library_top")

        runner.inject_card(1, BT12_047, "hand")
        runner.set_phase("Main")

        lib_before = len(runner.game.player1.library_cards)

        action = runner.find_action("Play Wormmon")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Library should be same size: 3 revealed, 0 taken, 3 returned
        assert snap.p1_library_size == lib_before - 3 + 3, (
            f"Library size should be unchanged. Before: {lib_before}, "
            f"After: {snap.p1_library_size}"
        )


@pytest.mark.behavioral
class TestBT12047InheritedDNA:
    """Inherited: [End of Your Turn] DNA digivolve from hand."""

    def test_inherited_eot_effect_exists(self, debug_runner):
        """BT12-047 should have an inherited OnEndTurn DNA digivolve effect.

        Engine gap #12 was RESOLVED (2026-03-14) with
        game.effect_dna_digivolve_from_hand(). The inherited effect should
        NOT be marked as BLOCKED.
        """
        runner = debug_runner(initial_memory=10)

        # Place BT12-047 under a Green Lv.4 to make it an inherited source
        perm = runner.place_on_field(1, [BT12_047, GREEN_LV4])

        # Get effects from BT12-047 (bottom card = inherited source)
        bt12_047_card = perm.card_sources[0]  # Bottom card
        assert bt12_047_card.card_id == BT12_047, (
            f"Bottom card should be BT12-047, got {bt12_047_card.card_id}"
        )
        effects = bt12_047_card.effect_list(None)
        eot_inherited = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False)
            and e.timing == EffectTiming.OnEndTurn
        ]
        assert len(eot_inherited) >= 1, (
            f"BT12-047 should have an inherited OnEndTurn effect (DNA digivolve). "
            f"Engine gap #12 is RESOLVED. "
            f"Effects: {[(getattr(e, '_effect_name', '?'), e.timing, getattr(e, 'is_inherited_effect', False)) for e in effects]}"
        )

    def test_inherited_eot_is_optional(self, debug_runner):
        """The inherited DNA digivolve should be optional ('may DNA digivolve')."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT12_047, GREEN_LV4])
        bt12_047_card = perm.card_sources[0]
        effects = bt12_047_card.effect_list(None)
        eot_inherited = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False)
            and e.timing == EffectTiming.OnEndTurn
        ]
        assert len(eot_inherited) >= 1, "Should have inherited OnEndTurn effect"
        assert eot_inherited[0].is_optional, (
            "Inherited DNA digivolve should be optional (card says 'may')"
        )

    def test_inherited_eot_triggers_dna_selection(self, debug_runner):
        """End of Turn should trigger DNA digivolve selection when
        inherited source is active and valid DNA targets exist."""
        runner = debug_runner(initial_memory=-3)
        game = runner.game

        # Stack: BT12-047 under Green Lv.4 (providing inherited effect)
        perm = runner.place_on_field(1, [BT12_047, GREEN_LV4])

        # Place a Blue Lv.4 as the other DNA material
        runner.place_on_field(1, [BLUE_LV4])

        # Put BT12-028 Paildramon (DNA: Blue Lv.4 + Green Lv.4) in hand
        runner.inject_card(1, PAILDRAMON_DNA, "hand")

        # Trigger end of turn
        runner.set_phase("End")
        game.phase_end()

        # Should have a pending selection for DNA digivolve
        ps = game.pending_selection
        assert ps is not None, (
            f"Inherited end-of-turn DNA digivolve should create a selection. "
            f"Phase: {runner.snapshot().phase}, actions: {runner.actions()}"
        )
