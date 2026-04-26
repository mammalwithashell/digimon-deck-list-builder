"""Behavioral tests for BT17-007 Agumon (Lv.3 Red Rookie).

Card text:
  Alt digivolve: from [Koromon] for cost 0.
  [Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its
      name, return 1 card with [Garurumon], [Greymon] or [Omnimon] in its
      name from your trash to the hand.
  Inherited: [End of Your Turn] This Digimon and any of your other Digimon
      may DNA digivolve into a Digimon card in the hand.

C# reference notes:
  - CanSelectCardCondition checks ContainsCardName only, NO IsDigimon check.
    Any card type (Digimon, Tamer, Option) with the right name qualifies.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT17007Agumon:
    """Tests for BT17-007 Agumon."""

    # ── Alt digivolve ────────────────────────────────────────────────

    def test_alt_digi_from_koromon(self, debug_runner):
        """Alt-digi effect should specify exact Koromon for cost 0."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-007"])
        top = perm.top_card
        effects = top.effect_list(None)

        alt_digi_effects = [
            e for e in effects
            if getattr(e, '_alt_digi_name', None) is not None
        ]
        assert len(alt_digi_effects) == 1, "Should have exactly 1 alt-digi effect"

        alt = alt_digi_effects[0]
        assert alt._alt_digi_name == "Koromon", "Alt-digi must be from Koromon"
        assert alt._alt_digi_cost == 0, "Alt-digi cost must be 0"
        assert alt._alt_digi_exact is True, "Alt-digi must be exact name match"

    # ── Start of Main Phase: trash return ────────────────────────────

    def test_trash_return_with_tai_kamiya_tamer(self, debug_runner):
        """With Tai Kamiya tamer on field and Greymon in trash,
        the start-of-main effect should activate and allow returning it."""
        runner = debug_runner(initial_memory=5)

        # Place BT17-007 Agumon on field
        perm = runner.place_on_field(1, ["BT17-007"])
        # Place Tai Kamiya tamer on field
        runner.place_on_field(1, ["BT1-085"])
        # Put a Greymon in trash
        runner.inject_card(1, "BT1-015", "trash")

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        start_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        assert len(start_effects) == 1, "Should have 1 OnStartMainPhase effect"

        eff = start_effects[0]
        # Condition should pass: have Tai Kamiya + Greymon in trash
        assert eff.can_use_condition({}), (
            "Condition should pass with Tai Kamiya tamer and Greymon in trash"
        )

        # Process the effect
        trash_before = list(game.player1.trash_cards)
        hand_before = list(game.player1.hand_cards)

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # The effect uses request_selection, which enters a selection phase.
        # Auto-resolve will pick the first legal selection.
        runner.auto_resolve()

        # Greymon should have moved from trash to hand
        trash_ids = [c.c_entity_base.card_id for c in game.player1.trash_cards]
        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "BT1-015" not in trash_ids, "Greymon should be removed from trash"
        assert "BT1-015" in hand_ids, "Greymon should be added to hand"

    def test_trash_return_no_tai_kamiya(self, debug_runner):
        """Without Tai Kamiya tamer, condition should fail."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-007"])
        # Put a Greymon in trash but no Tai Kamiya
        runner.inject_card(1, "BT1-015", "trash")

        top = perm.top_card
        effects = top.effect_list(None)
        start_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        assert len(start_effects) == 1

        eff = start_effects[0]
        assert not eff.can_use_condition({}), (
            "Condition should fail without Tai Kamiya tamer on field"
        )

    def test_trash_return_no_matching_card_in_trash(self, debug_runner):
        """With Tai Kamiya but no matching card in trash, condition should fail."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-007"])
        runner.place_on_field(1, ["BT1-085"])  # Tai Kamiya
        # No matching cards in trash

        top = perm.top_card
        effects = top.effect_list(None)
        start_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        eff = start_effects[0]
        assert not eff.can_use_condition({}), (
            "Condition should fail with no matching cards in trash"
        )

    def test_trash_return_garurumon(self, debug_runner):
        """Should also work with Garurumon in name."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-007"])
        runner.place_on_field(1, ["BT1-085"])  # Tai Kamiya
        runner.inject_card(1, "BT1-036", "trash")  # Garurumon

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        assert eff.can_use_condition({}), "Should find Garurumon in trash"

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "BT1-036" in hand_ids, "Garurumon should be returned to hand"

    def test_trash_return_omnimon(self, debug_runner):
        """Should also work with Omnimon in name."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-007"])
        runner.place_on_field(1, ["BT1-085"])  # Tai Kamiya
        runner.inject_card(1, "BT1-084", "trash")  # Omnimon

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        assert eff.can_use_condition({}), "Should find Omnimon in trash"

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert "BT1-084" in hand_ids, "Omnimon should be returned to hand"

    def test_trash_return_allows_non_digimon_with_matching_name(self, debug_runner):
        """C# reference has NO IsDigimon check -- any card type with matching
        name should be eligible. The filter should NOT restrict to is_digimon.

        This test verifies the filter function itself does not check is_digimon.
        We test this by directly inspecting the effect's condition and process logic.
        """
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-007"])
        runner.place_on_field(1, ["BT1-085"])  # Tai Kamiya

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        # Create a fake non-digimon card with "Greymon" in its name
        # by injecting a real Digimon and then marking it as non-digimon for test
        runner.inject_card(1, "BT1-015", "trash")  # Greymon
        greymon_card = game.player1.trash_cards[-1]

        # Verify this Greymon passes the condition normally
        assert eff.can_use_condition({}), "Condition should pass with Greymon in trash"

        # Also verify: if we manually set is_digimon=False on this card source,
        # the filter should STILL accept it (per C# reference).
        # Save original kind and restore in finally to avoid corrupting shared entity.
        from digimon_gym.engine.data.enums import CardKind
        orig_kind = greymon_card.c_entity_base.card_kind
        try:
            greymon_card.c_entity_base.card_kind = CardKind.Option  # fake it as Option
            assert greymon_card.is_digimon is False, "Sanity: card should now not be digimon"

            # Condition should STILL pass (C# has no IsDigimon check)
            assert eff.can_use_condition({}), (
                "Condition should still pass for non-Digimon cards with matching name "
                "(C# reference does not check IsDigimon)"
            )
        finally:
            greymon_card.c_entity_base.card_kind = orig_kind

    def test_trash_return_with_combined_tamer_name(self, debug_runner):
        """'Tai Kamiya & Matt Ishida' (BT5-093) contains 'Tai Kamiya'
        in its name, so it should satisfy the Tai Kamiya tamer condition."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-007"])
        # BT5-093 is "Tai Kamiya & Matt Ishida" combined tamer
        runner.place_on_field(1, ["BT5-093"])
        runner.inject_card(1, "BT1-015", "trash")  # Greymon

        top = perm.top_card
        effects = top.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]
        assert eff.can_use_condition({}), (
            "Combined 'Tai Kamiya & Matt Ishida' tamer should satisfy the condition"
        )

    # ── Inherited: End of Turn DNA digivolve ─────────────────────────

    def test_inherited_dna_effect_properties(self, debug_runner):
        """Inherited DNA digivolve effect should be optional, inherited,
        and trigger at End of Turn."""
        runner = debug_runner(initial_memory=5)

        # Place BT17-007 as a digivolution source under another Digimon
        perm = runner.place_on_field(1, ["BT17-007", "BT1-015"])  # Agumon under Greymon

        bottom_card = perm.card_sources[0]  # BT17-007 Agumon
        effects = bottom_card.effect_list(None)
        eot_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ]
        assert len(eot_effects) == 1, "Should have exactly 1 inherited OnEndTurn effect"

        eot = eot_effects[0]
        assert eot.is_inherited_effect, "Must be inherited"
        assert eot.is_optional, "DNA digivolve from hand must be optional"

    def test_inherited_dna_condition_needs_other_digimon(self, debug_runner):
        """Condition should fail if there is no other Digimon on field."""
        runner = debug_runner(initial_memory=5)

        # Only one Digimon on field (the one with BT17-007 inherited)
        perm = runner.place_on_field(1, ["BT17-007", "BT1-015"])

        # Put a Digimon card with dna_costs in hand
        runner.inject_card(1, "BT17-078", "hand")  # Omnimon (has DNA costs)

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        # No other Digimon on field, condition should fail
        assert not eot.can_use_condition({}), (
            "Condition should fail with only 1 Digimon on field (need another for DNA)"
        )

    def test_inherited_dna_condition_needs_hand_cards(self, debug_runner):
        """Condition should fail if hand is empty (no DNA target in hand)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT17-007", "BT1-015"])
        runner.place_on_field(1, ["ST1-07"])  # another Digimon

        # Clear the hand
        runner.clear_zone(1, "hand")

        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ][0]

        assert not eot.can_use_condition({}), (
            "Condition should fail with empty hand"
        )
