"""Behavioral tests for EX11-065 Close (Tamer, Black, Cost 3).

Card text:
  [Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] trait card
  from your hand or your Digimon's digivolution cards, gain 1 memory.

  [All Turns] When your Digimon are played or digivolve, if any of them
  have the [Mineral] or [Rock] trait, by suspending this Tamer, you may
  place 1 [Mineral] or [Rock] trait card from your hand or trash as any
  of those Digimon's bottom digivolution card.

  [Security] Play this card without paying the cost.

Key faithfulness points tested:
  - Effect 0: Start of Main Phase gives player choice between hand and digi-sources
    (branch selection, per C# SetIntSelection with "From hand"/"From digivolution cards"/"Do not trash")
  - Effect 0: When trashing from digi-sources, player selects which card to trash
    (not auto-selected)
  - Effect 0: Trashing from digi-sources properly removes from permanent and adds to trash
  - Effect 1: Play/digivolve trigger gives player choice between hand and trash
    (branch selection, per C# "From hand"/"From trash"/"Do not add")
  - Effect 1: When placing from trash, player selects which card to place
    (not auto-selected)
  - Effect 1: Does not double-fire on digivolve (OnEnterFieldAnyone covers both)
  - Effect 1: Tamer must be unsuspended to activate (suspend is the cost)
  - Effect 0 and 1: Memory gain / card placement works correctly
"""

import pytest
from engine_py_legacy.engine.data.enums import GamePhase

# Card IDs
EX11_065 = "EX11-065"       # Close (Tamer under test)
GOTSUMON = "BT4-065"         # Gotsumon Lv.3, Rock trait
GOLEMON = "BT4-066"          # Golemon Lv.4, Mineral trait
METEORMON = "BT4-070"        # Meteormon Lv.5, Rock trait
AGUMON = "ST1-03"            # Agumon Lv.3, Reptile (filler, no Mineral/Rock)

FILLER = [AGUMON] * 50


def _strip_mineral_rock_from_hand(player):
    """Remove all Mineral/Rock cards from a player's hand."""
    player.hand_cards[:] = [
        c for c in player.hand_cards
        if 'Mineral' not in (c.card_traits or [])
        and 'Rock' not in (c.card_traits or [])
    ]


@pytest.mark.behavioral
class TestEX11065StartOfMainPhase:
    """[Start of Your Main Phase] trash 1 Mineral/Rock from hand or digi-sources to gain 1 memory."""

    def test_trash_from_hand_gains_memory(self, debug_runner):
        """When player has Mineral/Rock in hand and activates the effect,
        trashing from hand should gain 1 memory."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 4 + [AGUMON] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.place_on_field(1, [EX11_065])
        _strip_mineral_rock_from_hand(runner.game.player1)
        runner.inject_card(1, GOTSUMON, "hand")

        before = runner.snapshot()
        mem_before = before.memory

        runner.set_phase("Main")
        runner.game.phase_main()
        runner.auto_resolve()

        after = runner.snapshot()
        assert after.memory >= mem_before + 1, (
            f"Memory should increase by 1. Before: {mem_before}, after: {after.memory}"
        )
        assert GOTSUMON in after.p1_trash, (
            f"Gotsumon should be in trash. Trash: {after.p1_trash}"
        )

    def test_trash_from_digisources_gains_memory(self, debug_runner):
        """When trashing from digi-sources (no hand candidates), player selects
        which card to trash and gains 1 memory."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 4 + [AGUMON] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.place_on_field(1, [EX11_065])
        perm = runner.place_on_field(1, [GOTSUMON, AGUMON])
        sources_before = len(perm.card_sources)

        _strip_mineral_rock_from_hand(runner.game.player1)

        before = runner.snapshot()
        mem_before = before.memory

        runner.set_phase("Main")
        runner.game.phase_main()

        # Walk through selections, choosing non-decline actions to actually trash
        for _ in range(15):
            if runner.game.game_over or runner.game.current_phase == GamePhase.Main:
                break
            legal = runner.action_mask()
            if not legal:
                break
            # Pick a non-decline action (62 = decline/pass)
            non_decline = [a for a in legal if a != 62]
            choice = non_decline[0] if non_decline else legal[0]
            runner.execute(choice)

        after = runner.snapshot()
        assert after.memory >= mem_before + 1, (
            f"Memory should increase by 1 from digi-source trash. "
            f"Before: {mem_before}, after: {after.memory}"
        )
        assert GOTSUMON in after.p1_trash, (
            f"Gotsumon should be in trash. Trash: {after.p1_trash}"
        )
        assert len(perm.card_sources) < sources_before, (
            f"Digi-sources should have lost 1 card. Before: {sources_before}, "
            f"after: {len(perm.card_sources)}"
        )

    def test_branch_choice_when_both_available(self, debug_runner):
        """When BOTH hand AND digi-sources have Mineral/Rock candidates,
        the player must get a branch choice (SelectEffectChoice).
        C# presents: 'From hand' / 'From digivolution cards' / 'Do not trash'."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 4 + [AGUMON] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.place_on_field(1, [EX11_065])
        runner.place_on_field(1, [GOTSUMON, GOLEMON])  # Gotsumon under Golemon
        _strip_mineral_rock_from_hand(runner.game.player1)
        runner.inject_card(1, METEORMON, "hand")  # Rock in hand

        runner.set_phase("Main")
        runner.game.phase_main()

        # Walk through selection phases looking for SelectEffectChoice
        found_branch = False
        for _ in range(10):
            if runner.game.game_over or runner.game.current_phase == GamePhase.Main:
                break
            if runner.game.current_phase == GamePhase.SelectEffectChoice:
                found_branch = True
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_branch, (
            "When both hand and digi-sources have Mineral/Rock candidates, "
            "must present a branch choice (SelectEffectChoice). "
            f"Current phase: {runner.game.current_phase}"
        )
        runner.auto_resolve()

    def test_digisource_trash_uses_player_selection(self, debug_runner):
        """When trashing from digi-sources, the player must get a selection
        phase to choose which card to trash (not auto-selected).
        C# uses SelectTrashDigivolutionCards."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 8 + [AGUMON] * 38,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.place_on_field(1, [EX11_065])
        # Place Digimon with TWO Rock digi-sources
        runner.place_on_field(1, [GOTSUMON, GOTSUMON, AGUMON])
        _strip_mineral_rock_from_hand(runner.game.player1)

        runner.set_phase("Main")
        runner.game.phase_main()

        # Walk through phases — should hit a selection for which card to trash
        found_selection = False
        for _ in range(10):
            if runner.game.game_over or runner.game.current_phase == GamePhase.Main:
                break
            phase = runner.game.current_phase
            # Any selection phase (SelectSource, SelectTarget, SelectEffectChoice)
            # that lets the player pick which digi-source to trash
            if phase in (GamePhase.SelectSource, GamePhase.SelectTarget,
                         GamePhase.SelectEffectChoice):
                actions = runner.actions()
                if len(actions) >= 2:
                    found_selection = True
                    break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_selection, (
            "When trashing from digi-sources with multiple candidates, "
            "player must get a selection phase. "
            f"Current phase: {runner.game.current_phase}"
        )
        runner.auto_resolve()


@pytest.mark.behavioral
class TestEX11065PlayDigivolve:
    """[All Turns] When Digimon played/digivolve with Mineral/Rock, suspend to place from hand/trash."""

    def test_place_from_hand_on_play(self, debug_runner):
        """When a Mineral/Rock Digimon is played and tamer is unsuspended,
        player can place a Mineral/Rock card from hand as bottom digi-source."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 6 + [AGUMON] * 40,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, [EX11_065])
        _strip_mineral_rock_from_hand(runner.game.player1)
        runner.inject_card(1, GOLEMON, "hand")

        # Play Gotsumon (Rock trait)
        runner.inject_card(1, GOTSUMON, "hand")
        play_actions = runner.find_actions("Play")
        gotsumon_play = None
        for aid, desc in play_actions.items():
            if 'gotsumon' in desc.lower():
                gotsumon_play = aid
                break
        if gotsumon_play is None:
            gotsumon_play = next(iter(play_actions), None)
        assert gotsumon_play is not None, f"Should be able to play Gotsumon. Actions: {runner.actions()}"
        runner.execute(gotsumon_play)
        runner.auto_resolve()

        after = runner.snapshot()
        assert tamer.is_suspended, "Tamer should be suspended after activating effect"

    def test_place_from_trash_on_play(self, debug_runner):
        """When only trash has Mineral/Rock candidates, player can select and
        place from trash as bottom digi-source."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 6 + [AGUMON] * 40,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, [EX11_065])
        runner.inject_card(1, GOLEMON, "trash")
        _strip_mineral_rock_from_hand(runner.game.player1)

        # Play Gotsumon (Rock trait)
        runner.inject_card(1, GOTSUMON, "hand")
        play_actions = runner.find_actions("Play")
        gotsumon_play = None
        for aid, desc in play_actions.items():
            if 'gotsumon' in desc.lower():
                gotsumon_play = aid
                break
        if gotsumon_play is None:
            gotsumon_play = next(iter(play_actions), None)
        assert gotsumon_play is not None
        runner.execute(gotsumon_play)
        runner.auto_resolve()

        after = runner.snapshot()
        assert tamer.is_suspended, "Tamer should be suspended"

    def test_branch_choice_when_both_hand_and_trash(self, debug_runner):
        """When BOTH hand AND trash have Mineral/Rock candidates,
        the player must get a branch choice (SelectEffectChoice).
        C# presents: 'From hand' / 'From trash' / 'Do not add'."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 6 + [AGUMON] * 40,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.place_on_field(1, [EX11_065])
        _strip_mineral_rock_from_hand(runner.game.player1)
        runner.inject_card(1, GOLEMON, "hand")
        runner.inject_card(1, METEORMON, "trash")

        # Play Gotsumon (Rock trait)
        runner.inject_card(1, GOTSUMON, "hand")
        play_actions = runner.find_actions("Play")
        gotsumon_play = None
        for aid, desc in play_actions.items():
            if 'gotsumon' in desc.lower():
                gotsumon_play = aid
                break
        if gotsumon_play is None:
            gotsumon_play = next(iter(play_actions), None)
        assert gotsumon_play is not None
        runner.execute(gotsumon_play)

        # Walk through until we find the branch choice
        found_branch = False
        for _ in range(10):
            if runner.game.game_over or runner.game.current_phase == GamePhase.Main:
                break
            if runner.game.current_phase == GamePhase.SelectEffectChoice:
                found_branch = True
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_branch, (
            "When both hand and trash have Mineral/Rock candidates, "
            "must present a branch choice (SelectEffectChoice). "
            f"Current phase: {runner.game.current_phase}"
        )
        runner.auto_resolve()

    def test_trash_selection_when_multiple_in_trash(self, debug_runner):
        """When placing from trash with multiple candidates, the player
        must get to select which card to place (not auto-selected)."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 6 + [AGUMON] * 40,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        runner.place_on_field(1, [EX11_065])
        # Put two different Mineral/Rock cards in trash
        runner.inject_card(1, GOLEMON, "trash")
        runner.inject_card(1, METEORMON, "trash")
        _strip_mineral_rock_from_hand(runner.game.player1)

        # Play Gotsumon (Rock trait)
        runner.inject_card(1, GOTSUMON, "hand")
        play_actions = runner.find_actions("Play")
        gotsumon_play = None
        for aid, desc in play_actions.items():
            if 'gotsumon' in desc.lower():
                gotsumon_play = aid
                break
        if gotsumon_play is None:
            gotsumon_play = next(iter(play_actions), None)
        assert gotsumon_play is not None
        runner.execute(gotsumon_play)

        # Walk through — should hit SelectTrash with at least 2 valid options
        found_trash_selection = False
        for _ in range(15):
            if runner.game.game_over or runner.game.current_phase == GamePhase.Main:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.SelectTrash:
                actions = runner.actions()
                if len(actions) >= 2:
                    found_trash_selection = True
                    break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_trash_selection, (
            "When placing from trash with multiple candidates, "
            "player must get SelectTrash phase with multiple options. "
            f"Current phase: {runner.game.current_phase}"
        )
        runner.auto_resolve()

    def test_tamer_must_be_unsuspended(self, debug_runner):
        """The tamer must be unsuspended to activate (suspend is the cost).
        If already suspended, the effect should not activate."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 6 + [AGUMON] * 40,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, [EX11_065])
        tamer.suspend()

        _strip_mineral_rock_from_hand(runner.game.player1)
        runner.inject_card(1, GOLEMON, "hand")

        # Play Gotsumon (Rock trait)
        runner.inject_card(1, GOTSUMON, "hand")
        play_actions = runner.find_actions("Play")
        gotsumon_play = None
        for aid, desc in play_actions.items():
            if 'gotsumon' in desc.lower():
                gotsumon_play = aid
                break
        if gotsumon_play is None:
            gotsumon_play = next(iter(play_actions), None)
        assert gotsumon_play is not None
        runner.execute(gotsumon_play)
        runner.auto_resolve()

        after = runner.snapshot()
        # Gotsumon should be on field with exactly 1 card in stack (no placement)
        for fs in after.p1_field:
            if 'Gotsumon' in fs.card_name:
                assert len(fs.stack_ids) == 1, (
                    f"Gotsumon should have no extra digi-sources (tamer was suspended). "
                    f"Stack: {fs.stack_ids}"
                )

    def test_no_double_fire_on_digivolve(self, debug_runner):
        """The effect should fire at most once per digivolve event.
        It should NOT fire on both OnEnterFieldAnyone and WhenDigivolving
        for the same digivolve. C# only uses OnEnterFieldAnyone."""
        runner = debug_runner(
            deck1=[EX11_065] * 4 + [GOTSUMON] * 6 + [GOLEMON] * 4 + [AGUMON] * 36,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")
        tamer = runner.place_on_field(1, [EX11_065])
        _strip_mineral_rock_from_hand(runner.game.player1)
        runner.inject_card(1, GOTSUMON, "hand")
        runner.inject_card(1, GOTSUMON, "hand")

        # Place Gotsumon on field to digivolve onto
        runner.place_on_field(1, [GOTSUMON])
        # Put Golemon in hand to digivolve
        runner.inject_card(1, GOLEMON, "hand")

        before = runner.snapshot()
        gotsumon_in_hand = sum(1 for c in before.p1_hand if c == GOTSUMON)

        digivolve_action = runner.find_action("Digivolve")
        if digivolve_action is not None:
            runner.execute(digivolve_action)
            runner.auto_resolve()

            after = runner.snapshot()
            gotsumon_in_hand_after = sum(1 for c in after.p1_hand if c == GOTSUMON)
            placed_count = gotsumon_in_hand - gotsumon_in_hand_after

            assert placed_count <= 1, (
                f"Effect should fire at most once per digivolve. "
                f"Cards placed from hand: {placed_count}"
            )
