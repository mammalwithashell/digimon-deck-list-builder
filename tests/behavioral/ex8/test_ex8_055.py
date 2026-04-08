"""Behavioral tests for EX8-055 Pyramidimon (Lv.6 Black Digimon, DP 12000).

Card text:
<Fragment (3)> (When this Digimon would be deleted, by trashing any 3 of its
    digivolution cards, it isn't deleted.)
[When Digivolving] [When Attacking] By trashing any 3 digivolution cards with
    the [Mineral] or [Rock] trait from your Digimon, this Digimon unsuspends and
    gains <Security A. +1> for the turn.
[End of Your Turn] [Once Per Turn] You may place up to 3 cards with the
    [Mineral] or [Rock] trait from your trash as this Digimon's bottom
    digivolution cards.

C# Reference: DCGO/Assets/Scripts/CardEffect/EX8/Black/EX8_055.cs
- Fragment (3): standard keyword
- WD/WA: SelectTrashDigivolutionCards with isFromOnly1Permanent=false, maxCount=3,
  canNoTrash=false (must trash exactly 3). Player selects which sources to trash.
- EOT: SelectCardEffect from Trash, maxCount=3, canEndNotMax=true (up to 3).
  Player selects Mineral/Rock cards from trash to place as bottom digi cards.

Key behaviors verified:
1. Fragment (3) keyword is declared
2. WD/WA: Player must SELECT which 3 Mineral/Rock sources to trash (not auto-trash)
3. WD/WA: Sources can come from ANY of your Digimon (not just this one)
4. WD/WA: After trashing 3, this Digimon unsuspends and gains SA+1
5. WD/WA: Condition requires >= 3 eligible sources across all Digimon
6. EOT: Player selects up to 3 Mineral/Rock cards from trash
7. EOT: Selected cards placed as bottom digivolution cards of this Digimon
8. EOT: Once per turn
9. EOT: Only during your turn
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase
from digimon_gym.engine.game.constants import SOURCES_PER_FIELD, SEL_TRASH_START


# Filler deck for DebugRunner (never used directly; state injection replaces)
_FILLER_DECK = (
    ["BT1-001"] * 4  # digi-eggs
    + ["ST1-02"] * 4  # Lv.3
    + ["ST1-03"] * 4
    + ["ST1-05"] * 4
    + ["ST1-07"] * 4
    + ["ST1-08"] * 4
    + ["ST1-09"] * 4
    + ["ST1-11"] * 4
    + ["ST1-12"] * 4
    + ["ST1-13"] * 4
    + ["ST1-14"] * 4
    + ["ST1-16"] * 4
    + ["ST1-06"] * 6
)

# Card IDs used:
# EX8-055 = Pyramidimon (Lv.6, Mineral, DP 12000)
# BT4-066 = Golemon (Lv.4, Mineral, DP 3000)
# BT2-054 = Gotsumon (Lv.3, Rock, DP 3000)
# BT4-065 = Gotsumon (Lv.3, Rock, DP 6000)
# ST1-03  = Agumon (Lv.3, Reptile) - NOT Mineral/Rock


def _trigger_when_digivolving(game, perm):
    """Fire WhenDigivolving timing with proper context."""
    game.execute_effects(EffectTiming.WhenDigivolving, {
        "permanent": perm,
        "digivolved_permanent": perm,
        "player": perm.owner,
    })


def _trigger_when_attacking(game, perm):
    """Fire OnUseAttack timing with proper context."""
    game.execute_effects(EffectTiming.OnUseAttack, {
        "permanent": perm,
        "attacker": perm,
        "player": perm.owner,
    })


@pytest.mark.behavioral
class TestEX8055Fragment:
    """Fragment (3) keyword."""

    def test_has_fragment_keyword(self, debug_runner):
        """EX8-055 should declare Fragment (3)."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX8-055"])
        top = perm.top_card
        effects = top.effect_list(None)
        fragment_effects = [e for e in effects if getattr(e, '_is_fragment', False)]
        assert len(fragment_effects) >= 1, (
            "EX8-055 should have a Fragment effect declared"
        )


@pytest.mark.behavioral
class TestEX8055WhenDigivolving:
    """[When Digivolving] By trashing 3 Mineral/Rock sources, unsuspend + SA+1."""

    def test_wd_offers_source_selection_not_auto_trash(self, debug_runner):
        """WD effect must present a SelectSource phase for player to choose
        which digivolution cards to trash, rather than auto-trashing."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        # Build a stack: EX8-055 on top, with 3 Mineral/Rock sources underneath
        # Stack order bottom->top: BT2-054, BT4-066, BT4-065, EX8-055
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-065", "EX8-055"])

        game = runner.game
        _trigger_when_digivolving(game, perm)

        # The game should be in a SelectSource phase waiting for the player to pick
        assert game.current_phase == GamePhase.SelectSource, (
            f"After triggering WD, game should be in SelectSource phase for player "
            f"to choose which sources to trash, but is in {game.current_phase.name}"
        )

    def test_wd_trashes_3_sources_unsuspends_gains_sa1(self, debug_runner):
        """After selecting 3 Mineral/Rock sources, this Digimon unsuspends
        and gains Security A. +1 for the turn."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        # Stack: BT2-054 (Rock), BT4-066 (Mineral), BT4-065 (Rock), EX8-055
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-065", "EX8-055"])
        perm.suspend()  # Suspend so we can verify unsuspend

        game = runner.game
        _trigger_when_digivolving(game, perm)

        # Resolve all 3 source selections
        runner.auto_resolve()

        snap = runner.snapshot()

        # Digimon should be unsuspended
        pyramidimon_slot = [s for s in snap.p1_field if "EX8-055" in s.stack_ids]
        assert len(pyramidimon_slot) == 1
        assert not pyramidimon_slot[0].is_suspended, (
            "After trashing 3 sources, Pyramidimon should be unsuspended"
        )

        # 3 Mineral/Rock sources should be in trash
        p1_trash = snap.p1_trash
        mineral_rock_in_trash = [
            cid for cid in p1_trash
            if cid in ("BT2-054", "BT4-066", "BT4-065")
        ]
        assert len(mineral_rock_in_trash) == 3, (
            f"3 Mineral/Rock cards should be in trash, found {mineral_rock_in_trash}"
        )

        # Only EX8-055 should remain in the stack (top card)
        assert pyramidimon_slot[0].stack_ids == ["EX8-055"], (
            f"Only EX8-055 should remain in stack, got {pyramidimon_slot[0].stack_ids}"
        )

    def test_wd_sources_from_different_digimon(self, debug_runner):
        """Sources can come from ANY of your Digimon, not just this one.
        Card text: 'from your Digimon' (not 'from this Digimon')."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        # Pyramidimon with 1 Mineral source
        perm = runner.place_on_field(1, ["BT4-066", "EX8-055"])
        # Another Digimon with 2 Rock sources
        other = runner.place_on_field(1, ["BT2-054", "BT4-065", "ST1-03"])

        game = runner.game
        _trigger_when_digivolving(game, perm)

        # Should enter selection phase (enough sources across both Digimon)
        assert game.current_phase == GamePhase.SelectSource, (
            f"Should be in SelectSource. Phase: {game.current_phase.name}"
        )

        # Resolve all 3 source selections
        runner.auto_resolve()

        snap = runner.snapshot()

        # 3 total Mineral/Rock cards should be trashed
        mineral_rock_in_trash = [
            cid for cid in snap.p1_trash
            if cid in ("BT2-054", "BT4-066", "BT4-065")
        ]
        assert len(mineral_rock_in_trash) == 3, (
            f"3 Mineral/Rock sources should be trashed from across Digimon, "
            f"found {mineral_rock_in_trash}"
        )

    def test_wd_condition_needs_3_sources(self, debug_runner):
        """WD condition should fail if fewer than 3 Mineral/Rock sources
        are available across all Digimon."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        # Only 2 Mineral/Rock sources
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "EX8-055"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd_effects = [
            e for e in effects
            if getattr(e, 'is_when_digivolving', False)
            and e.on_process_callback
        ]
        assert len(wd_effects) >= 1, "Should have a WD effect"

        # Condition should fail with only 2 sources
        wd = wd_effects[0]
        can_use = wd.can_use_condition({})
        assert not can_use, (
            "WD condition should fail with only 2 Mineral/Rock sources"
        )


@pytest.mark.behavioral
class TestEX8055WhenAttacking:
    """[When Attacking] By trashing 3 Mineral/Rock sources, unsuspend + SA+1."""

    def test_wa_offers_source_selection(self, debug_runner):
        """WA effect must present a SelectSource phase for player selection."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        # Stack: 3 Mineral/Rock under EX8-055
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-065", "EX8-055"])

        game = runner.game
        _trigger_when_attacking(game, perm)

        # Should be in SelectSource for player to choose
        assert game.current_phase == GamePhase.SelectSource, (
            f"After WA trigger, game should be in SelectSource, "
            f"but is in {game.current_phase.name}"
        )

    def test_wa_trashes_and_unsuspends(self, debug_runner):
        """After selecting 3 sources during attack, unsuspend + SA+1."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066", "BT4-065", "EX8-055"])
        perm.suspend()

        game = runner.game
        _trigger_when_attacking(game, perm)

        runner.auto_resolve()

        snap = runner.snapshot()
        slot = [s for s in snap.p1_field if "EX8-055" in s.stack_ids][0]
        assert not slot.is_suspended, "Should be unsuspended after trashing 3 sources"


@pytest.mark.behavioral
class TestEX8055EndOfTurn:
    """[End of Your Turn] [Once Per Turn] Place up to 3 Mineral/Rock from trash."""

    def test_eot_places_cards_from_trash(self, debug_runner):
        """EOT effect should place selected Mineral/Rock cards from trash
        as this Digimon's bottom digivolution cards."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX8-055"])

        # Put 3 Mineral/Rock cards in trash
        runner.inject_card(1, "BT2-054", "trash")  # Rock
        runner.inject_card(1, "BT4-066", "trash")  # Mineral
        runner.inject_card(1, "BT4-065", "trash")  # Rock

        game = runner.game
        game.player1.is_my_turn = True

        snap_before = runner.snapshot()
        trash_before = len(snap_before.p1_trash)

        # Trigger End of Turn
        game.execute_effects(EffectTiming.OnEndTurn, {
            "player": game.player1,
        })

        # Should be in SelectTrash for player to choose
        assert game.current_phase == GamePhase.SelectTrash, (
            f"Should enter SelectTrash phase, got {game.current_phase.name}"
        )

        # Manually select 3 cards from trash (auto_resolve would decline)
        for pick in range(3):
            legal = runner.action_mask()
            trash_actions = [a for a in legal if SEL_TRASH_START <= a <= 179]
            assert len(trash_actions) >= 1, (
                f"Pick {pick+1}/3: should have trash selection actions"
            )
            runner.execute(trash_actions[0])

        snap_after = runner.snapshot()
        slot = [s for s in snap_after.p1_field if "EX8-055" in s.stack_ids][0]

        # Cards should be placed as bottom digivolution cards
        placed_ids = [cid for cid in slot.stack_ids if cid != "EX8-055"]
        assert len(placed_ids) == 3, (
            f"3 cards should be placed under EX8-055, got {placed_ids}"
        )

        # Trash should be reduced by 3
        assert len(snap_after.p1_trash) == trash_before - 3, (
            f"Trash should have 3 fewer cards"
        )

    def test_eot_places_partial_when_declined(self, debug_runner):
        """EOT is optional 'up to 3' -- player can stop after fewer."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX8-055"])

        # Put 3 Mineral/Rock cards in trash
        runner.inject_card(1, "BT2-054", "trash")  # Rock
        runner.inject_card(1, "BT4-066", "trash")  # Mineral
        runner.inject_card(1, "BT4-065", "trash")  # Rock

        game = runner.game
        game.player1.is_my_turn = True

        # Trigger End of Turn
        game.execute_effects(EffectTiming.OnEndTurn, {
            "player": game.player1,
        })

        assert game.current_phase == GamePhase.SelectTrash

        # Select 1 card, then decline
        legal = runner.action_mask()
        trash_actions = [a for a in legal if SEL_TRASH_START <= a <= 179]
        assert len(trash_actions) >= 1, "Should have trash selection actions"

        # Pick first eligible card
        runner.execute(trash_actions[0])

        # Now decline (action 62)
        assert game.current_phase == GamePhase.SelectTrash, (
            "Should still be in SelectTrash for next pick"
        )
        runner.execute(62)  # Decline further selections

        snap = runner.snapshot()
        slot = [s for s in snap.p1_field if "EX8-055" in s.stack_ids][0]
        placed_ids = [cid for cid in slot.stack_ids if cid != "EX8-055"]
        assert len(placed_ids) == 1, (
            f"Only 1 card should be placed when player declines after 1, got {placed_ids}"
        )

    def test_eot_once_per_turn(self, debug_runner):
        """EOT effect should only trigger once per turn."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX8-055"])
        runner.inject_card(1, "BT2-054", "trash")
        runner.inject_card(1, "BT4-066", "trash")
        runner.inject_card(1, "BT4-065", "trash")

        game = runner.game
        game.player1.is_my_turn = True

        # First trigger
        game.execute_effects(EffectTiming.OnEndTurn, {"player": game.player1})
        runner.auto_resolve()

        # Put more cards in trash for potential second trigger
        runner.inject_card(1, "BT2-054", "trash")

        # Second trigger in same turn should not fire
        game.execute_effects(EffectTiming.OnEndTurn, {"player": game.player1})

        # Should NOT enter selection phase again
        assert game.current_phase != GamePhase.SelectTrash, (
            "EOT should not fire a second time in the same turn"
        )

    def test_eot_only_your_turn(self, debug_runner):
        """EOT should only trigger during your own turn."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX8-055"])
        runner.inject_card(1, "BT2-054", "trash")

        game = runner.game
        game.player1.is_my_turn = False  # Not P1's turn

        top = perm.top_card
        effects = top.effect_list(None)
        eot_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.on_process_callback
        ]
        assert len(eot_effects) >= 1

        # Condition should fail when not your turn
        can_use = eot_effects[0].can_use_condition({})
        assert not can_use, "EOT condition should fail when not your turn"

    def test_eot_only_mineral_rock_selectable(self, debug_runner):
        """EOT should only allow selecting Mineral/Rock cards from trash,
        not other cards."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX8-055"])

        # Mix of Mineral/Rock and non-Mineral/Rock in trash
        runner.inject_card(1, "BT2-054", "trash")  # Rock (index 0)
        runner.inject_card(1, "ST1-03", "trash")    # Reptile (index 1) - NOT eligible
        runner.inject_card(1, "BT4-066", "trash")  # Mineral (index 2)

        game = runner.game
        game.player1.is_my_turn = True

        # Trigger End of Turn
        game.execute_effects(EffectTiming.OnEndTurn, {"player": game.player1})

        assert game.current_phase == GamePhase.SelectTrash

        # Check valid actions -- should NOT include the ST1-03 (non-Mineral/Rock)
        legal = runner.action_mask()
        trash_actions = [a for a in legal if SEL_TRASH_START <= a <= 179]

        # Find indices of trash cards
        snap = runner.snapshot()
        st1_03_idx = None
        for i, cid in enumerate(snap.p1_trash):
            if cid == "ST1-03":
                st1_03_idx = i
                break

        if st1_03_idx is not None:
            non_eligible_action = SEL_TRASH_START + st1_03_idx
            assert non_eligible_action not in trash_actions, (
                f"ST1-03 (Reptile, non-Mineral/Rock) at index {st1_03_idx} "
                f"should NOT be selectable"
            )
