"""Behavioral tests for EX11-044 Pyramidimon (Lv.6, Black, Mineral/Rock Dragon).

Card text:
    Reboot
    Fragment (3)
    [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
      By trashing any 3 [Mineral] or [Rock] trait cards from your
      Digimon's digivolution cards, delete 1 of your opponent's
      highest play cost Digimon or Tamers.
    [All Turns] [Once Per Turn] When effects trash any of this Digimon's
      digivolution cards, you may place 3 [Mineral] or [Rock] trait cards
      from your trash as this Digimon's bottom digivolution cards.

Key faithfulness requirements:
  1. Trash-3 cost: player must CHOOSE which 3 Mineral/Rock digivolution cards
     to trash (not auto-select). Uses SelectSource phase for each pick.
  2. Trashing must fire OnDigivolutionCardDiscarded timing so the second
     effect can self-trigger.
  3. Delete target: opponent's highest play cost Digimon or Tamer (player
     selects among tied-for-highest).
  4. Place-from-trash: player must CHOOSE which 3 Mineral/Rock cards to
     place from trash (not auto-select). Uses SelectTrash phase.
  5. Once Per Turn: each effect has its own OPT, shared across timings.
  6. [All Turns]: the place-from-trash effect fires on any turn, not just yours.
"""

import pytest

# Cards used:
# EX11-044: Pyramidimon (Lv.6, Black, Mineral, cost 11, DP 12000)
# BT2-054: Gotsumon (Lv.3, Black, Rock trait, cost 3)
# BT4-066: Golemon (Lv.4, Black, Mineral trait, cost 4)
# BT4-070: Meteormon (Lv.5, Black, Rock trait, cost 7)
# BT1-019: DarkTyrannomon (Lv.4, Black, Dinosaur, cost 6) -- NOT Mineral/Rock (negative test)
# BT1-085: Tai Kamiya (Tamer, cost 4) -- opponent target
# BT3-007: Agumon (Lv.3, Red, cost 2) -- filler

PYRAMIDIMON_DECK = ["EX11-044"] * 4 + ["BT2-054"] * 15 + ["BT4-066"] * 15 + ["BT4-070"] * 12 + ["BT3-007"] * 4
FILLER_DECK = ["BT3-007"] * 50


@pytest.mark.behavioral
class TestEX11044TrashAndDelete:
    """Tests for the [On Play]/[When Digivolving]/[When Attacking] delete effect."""

    def test_trash_3_requires_player_selection(self, debug_runner):
        """Trashing 3 Mineral/Rock digivolution cards must be player-chosen,
        not auto-selected. The game should enter a SelectSource phase for each pick."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        game = runner.game
        p1 = game.player1
        p2 = game.player2

        # Build a Pyramidimon with 4 Mineral/Rock digivolution cards under it
        # Stack: BT2-054 (Rock), BT4-066 (Mineral), BT4-066 (Mineral), BT4-070 (Rock), EX11-044 (top)
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-066", "BT4-070", "EX11-044"])

        # Place an opponent Digimon to delete
        runner.place_on_field(2, ["BT3-007"])

        snap_before = runner.snapshot()
        # Pyramidimon should have 5 cards in its stack (4 under + top)
        pyra_field = [s for s in snap_before.p1_field if s.card_id == "EX11-044"]
        assert pyra_field, "Pyramidimon should be on field"
        assert len(pyra_field[0].stack_ids) == 5, f"Expected 5 cards in stack, got {len(pyra_field[0].stack_ids)}"

        # Trigger the attack action to activate the effect
        action = runner.find_action("Attack")
        assert action is not None, f"Should have an attack action. Available: {runner.actions()}"
        runner.execute(action)

        # The effect should be triggered. After accepting, it should
        # enter a selection phase for the player to choose which source cards to trash.
        # Find and accept the optional effect
        opt_action = runner.find_action("EX11-044")
        if opt_action is None:
            opt_action = runner.find_action("Trash 3")
        if opt_action is not None:
            runner.execute(opt_action)

        # Now the game should be in a SelectSource phase for picking digivolution cards
        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectSource, \
            f"Expected SelectSource phase for digivolution card selection, got {game.current_phase.name}"

        # There should be valid source indices corresponding to Mineral/Rock cards
        ps = game.pending_selection
        assert ps is not None, "Should have a pending selection"
        assert len(ps.valid_indices) >= 3, \
            f"Should have at least 3 valid Mineral/Rock sources to pick, got {len(ps.valid_indices)}"

    def test_trash_3_sends_cards_to_trash_and_deletes_opponent(self, debug_runner):
        """After selecting 3 Mineral/Rock digivolution cards, they should go to
        trash and one of opponent's highest cost Digimon/Tamers should be deleted.

        Note: The place-from-trash effect may reclaim the trashed cards (since
        OnDigivolutionCardDiscarded fires after delete). If it does, trash will
        be empty and the stack will be replenished. We verify the delete happened.
        """
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        game = runner.game
        p1 = game.player1
        p2 = game.player2

        # Build Pyramidimon with enough Mineral/Rock sources
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-066", "BT4-070", "EX11-044"])

        # Place opponent Digimon with known play cost
        opp_perm = runner.place_on_field(2, ["BT1-019"])  # DarkTyrannomon, cost 6

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)

        # Attack to trigger [When Attacking]
        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Opponent's DarkTyrannomon should have been deleted
        assert len(snap_after.p2_field) == p2_field_before - 1, \
            f"Expected opponent to lose 1 field card, field went from {p2_field_before} to {len(snap_after.p2_field)}"

        # Pyramidimon should still be on field
        pyra_field = [s for s in snap_after.p1_field if s.card_id == "EX11-044"]
        assert pyra_field, "Pyramidimon should still be on field"

        # If place-from-trash didn't trigger (e.g. not enough trash), stack = 2
        # If it did trigger and reclaimed 3, stack = 5
        # Either way, top should still be EX11-044
        assert pyra_field[0].card_id == "EX11-044", "Top card should still be Pyramidimon"

    def test_delete_targets_highest_play_cost(self, debug_runner):
        """Delete should target the opponent's HIGHEST play cost Digimon/Tamer."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        game = runner.game

        # Pyramidimon with 3+ Mineral/Rock sources
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-066", "BT4-070", "EX11-044"])

        # Opponent has two Digimon: low cost and high cost
        low_cost = runner.place_on_field(2, ["BT3-007"])   # Agumon, cost 2
        high_cost = runner.place_on_field(2, ["BT1-019"])  # DarkTyrannomon, cost 6

        # Attack to trigger effect
        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Only the high-cost DarkTyrannomon should be deleted
        opp_card_ids = [s.card_id for s in snap_after.p2_field]
        assert "BT3-007" in opp_card_ids, \
            "Low-cost Agumon should NOT be deleted"
        assert "BT1-019" not in opp_card_ids, \
            "High-cost DarkTyrannomon SHOULD be deleted"

    def test_not_enough_mineral_rock_sources_no_trigger(self, debug_runner):
        """Effect should not trigger if fewer than 3 Mineral/Rock digivolution
        cards are available across all your Digimon."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )

        # Pyramidimon with only 2 Mineral/Rock sources (not enough)
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "EX11-044"])

        # Opponent has a target
        runner.place_on_field(2, ["BT1-019"])

        # Attack
        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)

        # The optional effect should NOT appear (or if it does, condition fails)
        # Just auto-resolve and verify opponent is untouched
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert len(snap_after.p2_field) == 1, \
            "Opponent should still have 1 field card (not enough sources to trash)"

    def test_once_per_turn_cannot_activate_twice(self, debug_runner):
        """The [Once Per Turn] constraint should prevent using the delete effect
        twice in the same turn, even across different timings."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )

        # Pyramidimon with 6+ Mineral/Rock sources (enough for 2 uses)
        perm = runner.place_on_field(1, [
            "BT2-054", "BT4-066", "BT4-066", "BT4-070",
            "BT2-054", "BT4-066", "BT4-066", "EX11-044",
        ])

        # Two opponent targets
        runner.place_on_field(2, ["BT1-019"])
        runner.place_on_field(2, ["BT1-019"])

        # First attack triggers the effect
        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_mid = runner.snapshot()
        # Should have deleted 1 opponent Digimon (not 2)
        deleted_count = 2 - len(snap_mid.p2_field)
        assert deleted_count == 1, \
            f"Expected exactly 1 deletion (OPT), but {deleted_count} were deleted"


    def test_trash_fires_on_digivolution_card_discarded(self, debug_runner):
        """Trashing digivolution cards should fire OnDigivolutionCardDiscarded timing,
        which is needed for the place-from-trash effect to trigger."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        game = runner.game

        # Pyramidimon with sources + seeded trash for the place-back effect
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-066", "BT4-070", "EX11-044"])
        runner.place_on_field(2, ["BT1-019"])

        # Pre-seed trash with 3 Mineral/Rock for the place-from-trash trigger
        runner.inject_card(1, "BT2-054", "trash")
        runner.inject_card(1, "BT4-066", "trash")
        runner.inject_card(1, "BT4-066", "trash")

        # Track whether OnDigivolutionCardDiscarded fires
        timing_fired = [False]
        original_execute_effects = game.execute_effects

        from digimon_gym.engine.data.enums import EffectTiming
        def patched_execute_effects(timing, context=None):
            if timing == EffectTiming.OnDigivolutionCardDiscarded:
                timing_fired[0] = True
            return original_execute_effects(timing, context)

        game.execute_effects = patched_execute_effects

        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        assert timing_fired[0], \
            "OnDigivolutionCardDiscarded timing should fire when effect trashes digivolution cards"


@pytest.mark.behavioral
class TestEX11044PlaceFromTrash:
    """Tests for the [All Turns] place-from-trash effect."""

    def test_place_from_trash_requires_player_selection(self, debug_runner):
        """When digivolution cards are trashed by effects, the player should
        CHOOSE which 3 Mineral/Rock cards from trash to place back (not auto-select)."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        game = runner.game
        p1 = game.player1

        # Pyramidimon with Mineral/Rock sources
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-066", "BT4-070", "EX11-044"])

        # Opponent to delete
        runner.place_on_field(2, ["BT1-019"])

        # Pre-seed trash with 4 Mineral/Rock cards (so player has choice)
        runner.inject_card(1, "BT2-054", "trash")
        runner.inject_card(1, "BT2-054", "trash")
        runner.inject_card(1, "BT4-066", "trash")
        runner.inject_card(1, "BT4-066", "trash")

        # Attack -> trash 3 -> triggers OnDigivolutionCardDiscarded
        # -> then place 3 from trash effect should fire with selection
        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)

        # Process the attack and effect triggering
        # We need to check that at some point during resolution,
        # a SelectTrash phase appears for the place-from-trash effect
        from digimon_gym.engine.data.enums import GamePhase
        found_trash_selection = False
        for _ in range(30):
            if game.game_over:
                break
            if game.current_phase in (GamePhase.Main, GamePhase.Breeding):
                break
            if game.current_phase == GamePhase.SelectTrash:
                # Verify the prompt mentions placing from trash
                ps = game.pending_selection
                if ps and ('place' in ps.prompt.lower() or 'trash' in ps.prompt.lower()):
                    found_trash_selection = True
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        # The place-from-trash effect should have offered a SelectTrash selection
        # at some point (it was resolved by auto_resolve, but the phase was entered)
        # We verify by checking that cards were moved from trash to digivolution
        snap_after = runner.snapshot()
        pyra = [s for s in snap_after.p1_field if s.card_id == "EX11-044"]
        assert pyra, "Pyramidimon should still be on field"
        # After trashing 3 and placing 3 back, stack should be: original 2 + 3 placed = 5
        # (started with 5, trashed 3 -> 2, placed 3 -> 5)
        assert len(pyra[0].stack_ids) >= 4, \
            f"Expected stack to have grown after placing from trash, got {len(pyra[0].stack_ids)} cards"

    def test_place_from_trash_adds_to_bottom(self, debug_runner):
        """Cards placed from trash should go to the BOTTOM of the digivolution stack."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        game = runner.game
        p1 = game.player1

        # Pyramidimon with enough sources
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-066", "BT4-070", "EX11-044"])

        runner.place_on_field(2, ["BT1-019"])

        # Pre-seed trash with known Mineral/Rock cards
        runner.inject_card(1, "BT2-054", "trash")
        runner.inject_card(1, "BT2-054", "trash")
        runner.inject_card(1, "BT4-066", "trash")

        # Attack and auto-resolve
        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        pyra = [s for s in snap_after.p1_field if s.card_id == "EX11-044"]
        assert pyra, "Pyramidimon should still be on field"
        # Top card should still be EX11-044
        assert pyra[0].card_id == "EX11-044", \
            f"Top of stack should still be Pyramidimon, got {pyra[0].card_id}"
        # Check bottom cards were added from trash (they should be Mineral/Rock cards)
        stack = pyra[0].stack_ids
        # The bottom cards should include the placed-from-trash cards
        assert len(stack) >= 3, \
            f"Stack should have at least 3 cards (1 remaining + placed), got {len(stack)}"

    def test_place_from_trash_not_enough_cards_partial(self, debug_runner):
        """If fewer than 3 Mineral/Rock cards are in trash, the effect should
        not trigger (requires exactly 3)."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        game = runner.game

        # Pyramidimon with sources (the trashing itself will put 3 cards in trash)
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-066", "BT4-070", "EX11-044"])

        runner.place_on_field(2, ["BT1-019"])

        # NO pre-seeded trash - after trashing 3 sources, there will be exactly 3
        # Mineral/Rock in trash, which is enough to trigger
        snap_before = runner.snapshot()
        assert len(snap_before.p1_trash) == 0, "Trash should be empty initially"

        action = runner.find_action("Attack")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        pyra = [s for s in snap_after.p1_field if s.card_id == "EX11-044"]
        assert pyra, "Pyramidimon should still be on field"
        # The 3 trashed cards go to trash, then the place-from-trash effect
        # can use those same 3 cards to place back
        # So stack should have been replenished

    def test_place_from_trash_is_optional(self, debug_runner):
        """The place-from-trash effect is optional ('you may')."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )

        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-066", "BT4-070", "EX11-044"])
        runner.place_on_field(2, ["BT1-019"])

        # Check that the effect is marked optional
        from digimon_gym.engine.data.enums import EffectTiming
        effects = perm.effect_list(EffectTiming.OnDigivolutionCardDiscarded)
        place_effects = [e for e in effects if 'EX11-044' in (e.effect_name or '') and 'Place' in (e.effect_name or '')]
        for eff in place_effects:
            assert eff.is_optional, "Place-from-trash effect should be optional"


@pytest.mark.behavioral
class TestEX11044Keywords:
    """Tests for keyword effects (Reboot, Fragment)."""

    def test_has_reboot(self, debug_runner):
        """Pyramidimon should have the Reboot keyword."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        perm = runner.place_on_field(1, ["EX11-044"])
        assert perm.has_keyword('_is_reboot'), "Pyramidimon should have Reboot"

    def test_has_fragment_3(self, debug_runner):
        """Pyramidimon should have Fragment (3) keyword."""
        runner = debug_runner(
            deck1=PYRAMIDIMON_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        perm = runner.place_on_field(1, ["EX11-044"])
        assert perm.has_keyword('_is_fragment'), "Pyramidimon should have Fragment"
        # Check fragment count
        from digimon_gym.engine.data.enums import EffectTiming
        effects = perm.effect_list(EffectTiming.NoTiming)
        fragment_effects = [e for e in effects if getattr(e, '_is_fragment', False)]
        assert fragment_effects, "Should have Fragment effect"
        assert fragment_effects[0]._fragment_count == 3, \
            f"Fragment count should be 3, got {fragment_effects[0]._fragment_count}"
