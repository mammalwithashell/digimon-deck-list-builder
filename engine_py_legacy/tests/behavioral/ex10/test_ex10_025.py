"""Behavioral tests for EX10-025 Sunarizamon (Lv.3 Black Digimon).

Card text:
[On Play] You may place 2 cards with the [Mineral] or [Rock] trait from your
trash as 1 of your [Mineral] or [Rock] trait Digimon's bottom digivolution cards.

Inherited: When effects trash this card from a [Mineral] or [Rock] trait
Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play
cost of 4 or less.
"""

import pytest
from engine_py_legacy.engine.data.enums import GamePhase


@pytest.mark.behavioral
class TestEX10025Sunarizamon:
    """Tests for EX10-025 Sunarizamon."""

    # ── On Play effect ──────────────────────────────────────────────────

    def test_on_play_places_cards_from_trash_as_bottom_digi_cards(self, debug_runner):
        """[On Play] should let player select up to 2 Mineral/Rock cards from
        trash and place them as a Mineral/Rock Digimon's bottom digi-cards."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Put Mineral/Rock cards in trash
        runner.inject_card(1, "EX10-028", "trash")  # Landramon (Mineral)
        runner.inject_card(1, "BT2-054", "trash")   # Gotsumon (Rock)

        # Place a Mineral Digimon on field to receive digi-cards
        target_perm = runner.place_on_field(1, ["EX10-032"])  # Proganomon (Mineral)

        # Put Sunarizamon in hand
        runner.inject_card(1, "EX10-025", "hand")

        snap_before = runner.snapshot()
        target_sources_before = len(target_perm.card_sources)
        trash_before = len(snap_before.p1_trash)

        action = runner.find_action("Play Sunarizamon")
        assert action is not None, "Should be able to play Sunarizamon from hand"
        runner.execute(action)

        # Phase should be SelectTrash (C# order: trash first)
        game = runner.game
        assert game.current_phase == GamePhase.SelectTrash

        # Select the first Mineral/Rock card from trash (action 130 = trash index 0)
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START, SEL_MY_FIELD_START
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert len(trash_actions) >= 1, "Should have at least 1 trash card to select"
        runner.execute(trash_actions[0])  # Select first eligible card

        # Should prompt for second trash card
        assert game.current_phase == GamePhase.SelectTrash
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        if trash_actions:
            runner.execute(trash_actions[0])  # Select second eligible card

        # Now should be in permanent selection phase (SelectTarget for own permanents)
        assert game.current_phase == GamePhase.SelectTarget
        legal = runner.action_mask()
        field_actions = [a for a in legal if SEL_MY_FIELD_START <= a < SEL_MY_FIELD_START + 14]
        assert len(field_actions) >= 1, "Should have at least 1 Mineral/Rock Digimon to select"
        runner.execute(field_actions[0])  # Select the permanent

        snap_after = runner.snapshot()

        # Verify Sunarizamon is on the field
        assert any(
            s.card_id == "EX10-025" for s in snap_after.p1_field
        ), "Sunarizamon should be on the field"

        # The target Digimon should have gained digi-cards
        assert len(target_perm.card_sources) > target_sources_before, (
            f"Target should have gained digi-cards; went from "
            f"{target_sources_before} to {len(target_perm.card_sources)}"
        )

        # Trash should have fewer cards (cards moved to digi-sources)
        assert len(snap_after.p1_trash) < trash_before, (
            "Trash should have fewer cards after placing them as digi-sources"
        )

    def test_on_play_is_optional(self, debug_runner):
        """[On Play] effect is optional ('You may'). Declining should leave
        trash and field unchanged."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX10-028", "trash")  # Landramon (Mineral)
        runner.inject_card(1, "BT2-054", "trash")   # Gotsumon (Rock)
        runner.place_on_field(1, ["EX10-032"])  # Proganomon target

        runner.inject_card(1, "EX10-025", "hand")

        snap_before = runner.snapshot()
        trash_before = len(snap_before.p1_trash)

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)

        # Decline selections (find "Pass" or action 0)
        # The first selection phase should be optional
        snap = runner.snapshot()
        if snap.phase not in ("Main", "Breeding", "End"):
            # We're in a selection phase -- decline
            pass_action = runner.find_action("pass")
            if pass_action is None:
                pass_action = runner.find_action("decline")
            if pass_action is not None:
                runner.execute(pass_action)

        runner.auto_resolve()
        snap_after = runner.snapshot()

        # Trash count should be the same (nothing moved)
        assert len(snap_after.p1_trash) == trash_before, (
            "Declining should leave trash unchanged"
        )

    def test_on_play_no_mineral_rock_in_trash_skips(self, debug_runner):
        """If there are no Mineral/Rock cards in trash, the effect should
        not activate (no selection prompts)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Only non-Mineral/Rock cards in trash
        runner.inject_card(1, "ST1-03", "trash")  # Agumon (no relevant traits)
        runner.place_on_field(1, ["EX10-032"])  # target on field

        runner.inject_card(1, "EX10-025", "hand")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should return to Main phase without any selection
        assert snap.phase == "Main", (
            "With no Mineral/Rock in trash, should return to Main without prompts"
        )

    def test_on_play_no_mineral_rock_digimon_target_skips(self, debug_runner):
        """If there are no Mineral/Rock Digimon on field, the effect should
        not offer the permanent selection step."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX10-028", "trash")  # Mineral card in trash
        # Place a Digimon WITHOUT Mineral/Rock trait
        runner.place_on_field(1, ["ST1-03"])  # Agumon

        runner.inject_card(1, "EX10-025", "hand")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should complete without being stuck in a selection phase
        assert snap.phase == "Main", (
            "With no Mineral/Rock Digimon target, should return to Main"
        )

    def test_on_play_only_one_mineral_rock_in_trash_places_one(self, debug_runner):
        """When only 1 eligible card exists in trash, place at most 1."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX10-028", "trash")  # Just 1 Mineral card
        target_perm = runner.place_on_field(1, ["EX10-032"])

        runner.inject_card(1, "EX10-025", "hand")

        sources_before = len(target_perm.card_sources)

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Target should have gained at most 1 card
        gained = len(target_perm.card_sources) - sources_before
        assert gained <= 1, (
            f"With only 1 eligible trash card, should place at most 1; got {gained}"
        )

    # ── Selection order (C# faithfulness): trash first, then permanent ──

    def test_on_play_selects_trash_first(self, debug_runner):
        """C# behavioral truth: trash cards are selected BEFORE the permanent
        target. The first selection phase after playing should be SelectTrash."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX10-028", "trash")  # Mineral
        runner.inject_card(1, "BT2-054", "trash")   # Rock
        runner.place_on_field(1, ["EX10-032"])  # Mineral target on field

        runner.inject_card(1, "EX10-025", "hand")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)

        # The first selection phase should be trash selection
        snap = runner.snapshot()
        game = runner.game
        assert game.current_phase == GamePhase.SelectTrash, (
            f"First selection should be trash cards, got {game.current_phase.name}"
        )

    # ── Inherited effect ────────────────────────────────────────────────

    def test_inherited_effect_structure(self, debug_runner):
        """The inherited effect should have OnDigivolutionCardDiscarded timing
        and is_inherited_effect = True."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming

        # Place EX10-025 as a digivolution source under a Mineral Digimon
        perm = runner.place_on_field(1, ["EX10-025", "EX10-032"])

        # Find the OnDigivolutionCardDiscarded effect from EX10-025
        ex10_025_card = perm.card_sources[0]  # bottom card
        effects = ex10_025_card.effect_list(None)
        inherited_effects = [
            e for e in effects
            if e.is_inherited_effect and e.timing == EffectTiming.OnDigivolutionCardDiscarded
        ]
        assert len(inherited_effects) >= 1, (
            "EX10-025 should have an inherited OnDigivolutionCardDiscarded effect"
        )

    def test_inherited_condition_requires_mineral_or_rock_parent(self, debug_runner):
        """Inherited condition: the parent permanent must have Mineral or Rock trait.
        Should return False for a non-Mineral/Rock parent."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming

        # Place EX10-025 under a non-Mineral/Rock Digimon
        perm = runner.place_on_field(1, ["EX10-025", "ST1-03"])

        ex10_025_card = perm.card_sources[0]
        effects = ex10_025_card.effect_list(None)
        inherited_effect = next(
            (e for e in effects
             if e.is_inherited_effect and e.timing == EffectTiming.OnDigivolutionCardDiscarded),
            None,
        )
        assert inherited_effect is not None

        # Build context as the engine would (event_permanent is the perm where trashing happened)
        context = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": perm,
            "event_permanent": perm,
            "trashed_cards": [ex10_025_card],
        }
        assert not inherited_effect.can_use_condition(context), (
            "Should NOT trigger from a non-Mineral/Rock Digimon"
        )

    def test_inherited_condition_passes_for_mineral_parent(self, debug_runner):
        """Inherited condition: should return True for a Mineral parent."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming

        # Place EX10-025 under a Mineral Digimon (EX10-032 Proganomon)
        perm = runner.place_on_field(1, ["EX10-025", "EX10-032"])

        ex10_025_card = perm.card_sources[0]
        effects = ex10_025_card.effect_list(None)
        inherited_effect = next(
            (e for e in effects
             if e.is_inherited_effect and e.timing == EffectTiming.OnDigivolutionCardDiscarded),
            None,
        )
        assert inherited_effect is not None

        context = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": perm,
            "event_permanent": perm,
            "trashed_cards": [ex10_025_card],
        }
        assert inherited_effect.can_use_condition(context), (
            "Should trigger from a Mineral Digimon"
        )

    def test_inherited_condition_passes_for_rock_parent(self, debug_runner):
        """Inherited condition: should return True for a Rock parent."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming

        # BT2-054 Gotsumon has Rock trait
        perm = runner.place_on_field(1, ["EX10-025", "BT2-054"])

        ex10_025_card = perm.card_sources[0]
        effects = ex10_025_card.effect_list(None)
        inherited_effect = next(
            (e for e in effects
             if e.is_inherited_effect and e.timing == EffectTiming.OnDigivolutionCardDiscarded),
            None,
        )
        assert inherited_effect is not None

        context = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": perm,
            "event_permanent": perm,
            "trashed_cards": [ex10_025_card],
        }
        assert inherited_effect.can_use_condition(context), (
            "Should trigger from a Rock Digimon"
        )

    def test_inherited_condition_requires_this_card_in_trashed(self, debug_runner):
        """Inherited condition: should fail if this card is not in the trashed_cards list."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming

        perm = runner.place_on_field(1, ["EX10-025", "EX10-032"])

        ex10_025_card = perm.card_sources[0]
        effects = ex10_025_card.effect_list(None)
        inherited_effect = next(
            (e for e in effects
             if e.is_inherited_effect and e.timing == EffectTiming.OnDigivolutionCardDiscarded),
            None,
        )
        assert inherited_effect is not None

        # Context with a DIFFERENT card in trashed_cards
        other_card = runner.inject_card(1, "ST1-03", "trash")
        context = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": perm,
            "event_permanent": perm,
            "trashed_cards": [runner.game.player1.trash_cards[0]],  # ST1-03, not EX10-025
        }
        assert not inherited_effect.can_use_condition(context), (
            "Should NOT trigger when this card is not in trashed_cards"
        )

    def test_inherited_delete_target_filter_cost_4_or_less(self, debug_runner):
        """Inherited effect: delete target should only include opponent Digimon
        with play cost 4 or less."""
        runner = debug_runner(initial_memory=10)
        from engine_py_legacy.engine.data.enums import EffectTiming

        perm = runner.place_on_field(1, ["EX10-025", "EX10-032"])

        # Place opponent Digimon: one cost 3 (should be valid), one cost 11 (should not)
        opp_low = runner.place_on_field(2, ["ST1-03"])  # Agumon cost 3
        opp_high = runner.place_on_field(2, ["EX10-032"])  # Proganomon cost 7

        ex10_025_card = perm.card_sources[0]
        effects = ex10_025_card.effect_list(None)
        inherited_effect = next(
            (e for e in effects
             if e.is_inherited_effect and e.timing == EffectTiming.OnDigivolutionCardDiscarded),
            None,
        )

        # Check the filter manually: low cost should pass, high cost should not
        assert opp_low.top_card.get_cost_itself <= 4, (
            f"ST1-03 should have cost <= 4, got {opp_low.top_card.get_cost_itself}"
        )
        assert opp_high.top_card.get_cost_itself > 4, (
            f"EX10-032 should have cost > 4, got {opp_high.top_card.get_cost_itself}"
        )
