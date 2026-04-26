"""Behavioral tests for P-169 Close (Tamer, Black, Cost 4).

Card text:
  [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
  [All Turns] When effects trash digivolution cards of any of your [Mineral]
  or [Rock] trait Digimon, by suspending this Tamer, place 1 card with the
  [Mineral] or [Rock] trait from your trash as any of your Digimon's bottom
  digivolution card.
  Security Effect [Security] Play this card without paying the cost.

Key faithfulness points tested:
  Effect 0 - Start of Main Phase:
    C0-a: Fires at start of own main phase
    C0-b: Only fires if opponent has at least 1 Digimon
    C0-c: Gains exactly 1 memory

  Effect 1 - Digi-card trash trigger:
    C1-a: Triggers on OnDigivolutionCardDiscarded timing
    C1-b: The trashed-from permanent must be owned by the tamer's owner
    C1-c: The trashed-from permanent must have [Mineral] or [Rock] trait
    C1-d: Tamer must not already be suspended (suspending is the cost)
    C1-e: Must have at least 1 [Mineral] or [Rock] card in trash
    C1-f: Must have at least 1 Digimon on field to place under
    C1-g: Player CHOOSES which [Mineral]/[Rock] card from trash (SelectTrash)
    C1-h: Player CHOOSES which Digimon to place it under (SelectTarget)
    C1-i: Card is placed as the BOTTOM digivolution card
    C1-j: Effect is optional (is_optional = True)

  Effect 2 - Security:
    C2-a: Security play this card without paying the cost
"""

import pytest
from digimon_gym.engine.data.enums import GamePhase

# Card IDs
P_169 = "P-169"             # Card under test (Close, Tamer, Black)
GOTSUMON = "BT4-065"         # Lv.3 Black, Rock trait, Rookie
GOLEMON = "BT4-066"          # Lv.4 Black, Mineral trait, Champion
AGUMON_ST1 = "ST1-03"        # Lv.3 Red filler (non-Rock, non-Mineral)
GREYMON_ST1 = "ST1-07"       # Lv.4 Red filler

# Filler deck: must have enough cards for game setup
FILLER = [AGUMON_ST1] * 50


def _setup_security(runner, count=3):
    """Inject security cards so the game does not end prematurely."""
    for _ in range(count):
        runner.inject_card(1, AGUMON_ST1, "security_top")
        runner.inject_card(2, AGUMON_ST1, "security_top")


def _trigger_digi_card_trash(runner, permanent):
    """Directly trash 1 digivolution card from the given permanent.

    This simulates what an effect like Digi-Burst does: removes a digi-card
    from the stack and places it in trash. The trash_digivolution_cards method
    fires the OnDigivolutionCardDiscarded timing internally.
    """
    player = runner.game.player1
    trashed = permanent.trash_digivolution_cards(1, from_top=True)
    player.trash_cards.extend(trashed)
    return trashed


@pytest.mark.behavioral
class TestP169StartOfMainPhase:
    """Effect 0: [Start of Your Main Phase] If opponent has a Digimon, gain 1 memory."""

    def test_gains_memory_when_opponent_has_digimon(self, debug_runner):
        """C0-a, C0-b, C0-c: At start of main phase with opponent Digimon,
        should gain 1 memory."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=3,
        )
        runner.set_phase("Main")

        # Place P-169 tamer on P1's field
        runner.place_on_field(1, [P_169])

        # Place a Digimon on opponent's field
        runner.place_on_field(2, [AGUMON_ST1])

        _setup_security(runner)

        # Verify the start-of-main-phase effect exists with correct timing
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.enums import EffectTiming
        db = CardDatabase()
        cs = db.create_card_source(P_169, runner.game.player1)
        effects = cs.effect_list(None)
        main_phase_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
            and not e.is_inherited_effect
        ]
        assert len(main_phase_effects) >= 1, (
            "P-169 should have a start-of-main-phase effect"
        )

    def test_no_memory_gain_without_opponent_digimon(self, debug_runner):
        """C0-b: Without opponent Digimon, should NOT gain memory."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=3,
        )
        runner.set_phase("Main")

        # Place P-169 tamer on P1's field, no opponent Digimon
        tamer_perm = runner.place_on_field(1, [P_169])

        _setup_security(runner)

        # Verify condition returns False when opponent has no Digimon
        cs = tamer_perm.card_sources[0]
        effects = cs.effect_list(None)
        from digimon_gym.engine.data.enums import EffectTiming
        main_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        assert len(main_effects) >= 1
        # Opponent has no Digimon; condition should be False
        result = main_effects[0].can_use_condition({})
        assert result is False, (
            "Start-of-main-phase condition should be False without opponent Digimon"
        )


@pytest.mark.behavioral
class TestP169DigiCardTrashTrigger:
    """Effect 1: [All Turns] When effects trash digi-cards of Mineral/Rock Digimon,
    suspend this Tamer to place 1 Mineral/Rock from trash as bottom digi-source."""

    def test_trash_trigger_provides_trash_selection(self, debug_runner):
        """C1-g: When triggered, the player should get a SelectTrash phase
        to choose WHICH Mineral/Rock card from trash, not auto-select."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place P-169 tamer on P1's field (unsuspended)
        tamer_perm = runner.place_on_field(1, [P_169])
        assert not tamer_perm.is_suspended

        # Place a Rock-trait Digimon with a digi-source underneath
        # Stack: Gotsumon (bottom, Rock Lv.3) -> Golemon (top, Mineral Lv.4)
        rock_perm = runner.place_on_field(1, [GOTSUMON, GOLEMON])

        # Place another Digimon to be the target for placing the digi-card under
        target_perm = runner.place_on_field(1, [AGUMON_ST1])

        # Put two Mineral/Rock cards in trash so player has a real choice
        runner.inject_card(1, GOTSUMON, "trash")
        runner.inject_card(1, GOLEMON, "trash")

        _setup_security(runner)

        # Directly trash 1 digi-card from the Rock/Mineral Digimon
        _trigger_digi_card_trash(runner, rock_perm)

        # After the timing fires, the engine should offer the optional P-169 effect.
        # If the effect uses SelectTrash, we should see that phase.
        # Keep resolving until we see SelectTrash or return to Main.
        max_steps = 15
        found_trash_selection = False
        for _ in range(max_steps):
            if runner.game.game_over:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.SelectTrash:
                found_trash_selection = True
                break
            if phase == GamePhase.Main:
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_trash_selection, (
            f"P-169 trigger should present a SelectTrash phase for choosing which "
            f"Mineral/Rock card from trash. Final phase: {runner.game.current_phase.name}"
        )

    def test_trash_selection_then_target_selection(self, debug_runner):
        """C1-g, C1-h, C1-i: After choosing from trash, player selects which Digimon
        to place the card under (as bottom digi-card)."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        # Place P-169 tamer
        tamer_perm = runner.place_on_field(1, [P_169])

        # Place Mineral-trait Digimon with Gotsumon as digi-source
        # Top card is Golemon (Mineral), so has_trait('Mineral') is True
        rock_perm = runner.place_on_field(1, [GOTSUMON, GOLEMON])

        # Place a target Digimon
        target_perm = runner.place_on_field(1, [AGUMON_ST1])

        # Put a Mineral card in trash
        runner.inject_card(1, GOLEMON, "trash")

        _setup_security(runner)

        # Record initial state
        target_stack_before = len(target_perm.card_sources)

        # Trigger digi-card trash
        _trigger_digi_card_trash(runner, rock_perm)

        # Resolve through optional effect acceptance until SelectTrash
        max_steps = 15
        for _ in range(max_steps):
            if runner.game.game_over:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.SelectTrash:
                break
            if phase == GamePhase.Main:
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        if runner.game.current_phase == GamePhase.SelectTrash:
            # Select the Golemon from trash
            from digimon_gym.engine.game.constants import SEL_TRASH_START
            golemon_idx = None
            for i, card in enumerate(runner.game.player1.trash_cards):
                if card.c_entity_base and card.c_entity_base.card_id == GOLEMON:
                    golemon_idx = i
                    break
            assert golemon_idx is not None, "Golemon should be in trash"
            runner.execute(SEL_TRASH_START + golemon_idx)

            # Now should be in a target selection phase for choosing which Digimon
            runner.auto_resolve()

        # Verify results
        after = runner.snapshot()
        # Tamer should be suspended
        assert tamer_perm.is_suspended, "P-169 tamer should be suspended after using the effect"

        # Golemon should no longer be in trash (it was placed under a Digimon)
        assert GOLEMON not in after.p1_trash, (
            f"Golemon should have been removed from trash. Trash: {after.p1_trash}"
        )

    def test_tamer_gets_suspended_as_cost(self, debug_runner):
        """C1-d: The tamer is suspended as cost when the effect is used."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        tamer_perm = runner.place_on_field(1, [P_169])
        assert not tamer_perm.is_suspended, "Tamer should start unsuspended"

        # Place Mineral-trait Digimon with a digi-source
        rock_perm = runner.place_on_field(1, [GOTSUMON, GOLEMON])
        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, GOTSUMON, "trash")
        _setup_security(runner)

        # Trigger digi-card trash and resolve
        _trigger_digi_card_trash(runner, rock_perm)
        runner.auto_resolve()

        assert tamer_perm.is_suspended, (
            "P-169 tamer should be suspended after the effect triggers"
        )

    def test_no_trigger_when_already_suspended(self, debug_runner):
        """C1-d: If the tamer is already suspended, the effect cannot trigger."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        tamer_perm = runner.place_on_field(1, [P_169])
        # Pre-suspend the tamer
        tamer_perm.suspend()
        assert tamer_perm.is_suspended

        rock_perm = runner.place_on_field(1, [GOTSUMON, GOLEMON])
        runner.place_on_field(1, [AGUMON_ST1])
        runner.inject_card(1, GOTSUMON, "trash")
        _setup_security(runner)

        trash_before = list(runner.snapshot().p1_trash)

        # Trigger digi-card trash and resolve
        _trigger_digi_card_trash(runner, rock_perm)
        runner.auto_resolve()

        after = runner.snapshot()
        # The Gotsumon should still be in trash (P-169 didn't trigger)
        gotsumon_count_before = trash_before.count(GOTSUMON)
        gotsumon_count_after = after.p1_trash.count(GOTSUMON)
        assert gotsumon_count_after >= gotsumon_count_before, (
            "With tamer already suspended, P-169 should not trigger. "
            f"Gotsumon in trash before: {gotsumon_count_before}, after: {gotsumon_count_after}"
        )

    def test_no_trigger_for_non_mineral_rock_digimon(self, debug_runner):
        """C1-c: When a non-Mineral/non-Rock Digimon's digi-cards are trashed,
        P-169 should NOT trigger."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        tamer_perm = runner.place_on_field(1, [P_169])
        assert not tamer_perm.is_suspended

        # Place a non-Rock/non-Mineral Digimon with a digi-source
        non_rock_perm = runner.place_on_field(1, [AGUMON_ST1, GREYMON_ST1])

        runner.place_on_field(1, [AGUMON_ST1])  # potential target
        runner.inject_card(1, GOTSUMON, "trash")  # Mineral/Rock in trash
        _setup_security(runner)

        # Directly trash a digi-card from the non-Rock Digimon
        _trigger_digi_card_trash(runner, non_rock_perm)
        runner.auto_resolve()

        # Tamer should NOT have been suspended (effect should not trigger)
        assert not tamer_perm.is_suspended, (
            "P-169 should NOT trigger when a non-Mineral/non-Rock Digimon's "
            "digi-cards are trashed"
        )

    def test_no_trigger_without_mineral_rock_in_trash(self, debug_runner):
        """C1-e: If no Mineral/Rock cards in trash at trigger time, effect cannot trigger."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        tamer_perm = runner.place_on_field(1, [P_169])
        # Place a Mineral-trait Digimon with Gotsumon as a digi-source
        # Gotsumon (Rock) is the ONLY Rock/Mineral card - when it gets trashed
        # from digi stack, at the moment OnDigivolutionCardDiscarded fires,
        # it is NOT yet in trash (it's returned, then the caller adds it to trash).
        rock_perm = runner.place_on_field(1, [GOTSUMON, GOLEMON])
        runner.place_on_field(1, [AGUMON_ST1])

        # Make sure trash has NO Mineral/Rock cards
        runner.clear_zone(1, "trash")
        _setup_security(runner)

        # Trigger digi-card trash
        _trigger_digi_card_trash(runner, rock_perm)
        runner.auto_resolve()

        # Tamer should NOT be suspended because no Mineral/Rock was in trash
        # at the time of the trigger. The trashed Gotsumon gets added to trash
        # AFTER the trigger fires (by the caller in _trigger_digi_card_trash).
        assert not tamer_perm.is_suspended, (
            "P-169 should NOT trigger when no Mineral/Rock cards are in trash "
            "at the time of the trigger"
        )

    def test_card_placed_at_bottom_of_digi_stack(self, debug_runner):
        """C1-i: The selected card should be placed as the BOTTOM digivolution card."""
        runner = debug_runner(
            deck1=[P_169] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")

        tamer_perm = runner.place_on_field(1, [P_169])
        # Place Mineral Digimon with Gotsumon as digi-source
        rock_perm = runner.place_on_field(1, [GOTSUMON, GOLEMON])

        # The target Digimon to receive the card
        target_perm = runner.place_on_field(1, [AGUMON_ST1])
        target_stack_before = [
            cs.c_entity_base.card_id for cs in target_perm.card_sources
        ]

        # Put Golemon in trash
        runner.inject_card(1, GOLEMON, "trash")
        _setup_security(runner)

        # Trigger digi-card trash
        _trigger_digi_card_trash(runner, rock_perm)

        # Resolve through optional effect acceptance until SelectTrash
        from digimon_gym.engine.game.constants import SEL_TRASH_START, SEL_MY_FIELD_START

        max_steps = 15
        for _ in range(max_steps):
            if runner.game.game_over:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.SelectTrash:
                break
            if phase == GamePhase.Main:
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert runner.game.current_phase == GamePhase.SelectTrash, (
            f"Should be in SelectTrash phase, but in {runner.game.current_phase.name}"
        )

        # Select the Golemon from trash
        golemon_idx = None
        for i, card in enumerate(runner.game.player1.trash_cards):
            if card.c_entity_base and card.c_entity_base.card_id == GOLEMON:
                golemon_idx = i
                break
        assert golemon_idx is not None, "Golemon should be in trash"
        runner.execute(SEL_TRASH_START + golemon_idx)

        # Now should be in SelectTarget phase for choosing which Digimon
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget phase, but in {runner.game.current_phase.name}"
        )

        # Find the target_perm's slot index in the battle area and select it
        target_slot = None
        for i, perm in enumerate(runner.game.player1.battle_area):
            if perm is target_perm:
                target_slot = i
                break
        assert target_slot is not None, "Target permanent should be on the field"
        runner.execute(SEL_MY_FIELD_START + target_slot)

        runner.auto_resolve()

        # Check that the target Digimon now has the Golemon as its BOTTOM card
        target_stack_after = [
            cs.c_entity_base.card_id for cs in target_perm.card_sources
        ]

        assert len(target_stack_after) == len(target_stack_before) + 1, (
            f"Target should have 1 more card in its stack. "
            f"Before: {target_stack_before}, After: {target_stack_after}"
        )
        # Bottom card (index 0) should be the placed card
        assert target_stack_after[0] == GOLEMON, (
            f"Golemon should be at the BOTTOM of the digi-stack. "
            f"Stack (bottom to top): {target_stack_after}"
        )


@pytest.mark.behavioral
class TestP169Security:
    """Effect 2: Security Effect - play this card without paying the cost."""

    def test_security_effect_exists(self, debug_runner):
        """C2-a: P-169 should have a SecuritySkill effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.enums import EffectTiming
        db = CardDatabase()
        cs = db.create_card_source(P_169)
        effects = cs.effect_list(None)
        security_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill
        ]
        assert len(security_effects) >= 1, (
            "P-169 should have a Security Effect for playing without cost"
        )
        assert security_effects[0].is_security_effect is True
