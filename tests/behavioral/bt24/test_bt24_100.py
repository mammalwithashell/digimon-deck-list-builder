"""Behavioral tests for BT24-100 In-Between Theater (Option, White, Cost 3).

Card text:
While you have an [TS] trait Digimon or Tamer on the field, you can ignore
this card's color requirements.
[Main] Reveal the top 3 cards of your deck. Add 1 card with the [TS] trait
among them to the hand. Return the rest to the bottom of the deck.
Then, place this card in the battle area.
[Main] <Delay> Gain 2 memory.
[Security] Place this card in the battle area.
"""

import pytest


# BT24-043 Tapirmon: Green, [TS], Lv3
GREEN_TS_LV3 = "BT24-043"
# ST1-03 Agumon: Red, no TS trait
NON_TS_DIGIMON = "ST1-03"


@pytest.mark.behavioral
class TestBT24100InBetweenTheater:
    """Tests for BT24-100 In-Between Theater."""

    def test_color_ignore_with_ts_on_field(self, debug_runner):
        """Color bypass when you have a [TS] Digimon on field."""
        runner = debug_runner(initial_memory=10)

        # Place a [TS] Digimon on field
        runner.place_on_field(1, [GREEN_TS_LV3])

        card = runner.inject_card(1, "BT24-100", "hand")
        card.effect_list(None)

        assert card.match_color_requirement is False, (
            "Should bypass color requirement when [TS] Digimon is on field"
        )

    def test_color_enforced_without_ts_on_field(self, debug_runner):
        """Color enforced when no [TS] Digimon/Tamer on field."""
        runner = debug_runner(initial_memory=10)

        card = runner.inject_card(1, "BT24-100", "hand")
        card.effect_list(None)

        # No [TS] on field → enforce color
        assert card.match_color_requirement is True, (
            "Should enforce color requirement when no [TS] on field"
        )

    def test_main_reveal_adds_ts_card_to_hand(self, debug_runner):
        """[Main] reveals top 3, player selects 1 [TS] card to hand, rest to bottom."""
        runner = debug_runner(initial_memory=3)
        game = runner.game
        player = game.player1

        # Place a [TS] on field so color is ignored
        runner.place_on_field(1, [GREEN_TS_LV3])

        # Stack deck: put a [TS] card on top so reveal has something to pick
        # library_top inserts at index 0, so last insert is at the very top
        ts_card = runner.inject_card(1, GREEN_TS_LV3, "library_top")
        runner.inject_card(1, NON_TS_DIGIMON, "library_top")
        runner.inject_card(1, NON_TS_DIGIMON, "library_top")
        # Deck top is now: NON_TS, NON_TS, TS_CARD, ...

        hand_before = len(player.hand_cards)
        runner.inject_card(1, "BT24-100", "hand")
        runner.set_phase("Main")

        action = runner.find_action("In-Between Theater")
        if action is None:
            action = runner.find_action("BT24-100")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # Should have added 1 [TS] card to hand from the reveal
        # Hand: had hand_before + BT24-100 (played, gone) + 1 added from reveal
        hand_ids = [
            getattr(c, 'card_id', '') for c in player.hand_cards
        ]
        ts_in_hand = sum(1 for cid in hand_ids if cid == GREEN_TS_LV3)
        assert ts_in_hand >= 1, (
            f"Should have added a [TS] card to hand from reveal. Hand: {hand_ids}"
        )

    def test_main_places_option_in_battle_area(self, debug_runner):
        """[Main] places this card in the battle area after reveal (delay placement)."""
        runner = debug_runner(initial_memory=3)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [GREEN_TS_LV3])
        runner.inject_card(1, "BT24-100", "hand")
        runner.set_phase("Main")

        action = runner.find_action("In-Between Theater")
        if action is None:
            action = runner.find_action("BT24-100")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # BT24-100 should be in battle area (delay placement)
        ba_ids = [
            getattr(p.top_card, 'card_id', '') for p in player.battle_area
            if p.top_card
        ]
        assert "BT24-100" in ba_ids, (
            f"BT24-100 should be placed in battle area. BA: {ba_ids}"
        )

    def test_delay_gain_2_memory(self, debug_runner):
        """<Delay> trashes this card and gains 2 memory."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT24-100 in battle area as if it was played last turn
        perm = runner.place_on_field(1, ["BT24-100"], turn_played=0)

        # Advance to a new turn so delay is available (not the placing turn)
        game.turn_count = 2
        runner.set_phase("Main")

        mem_before = game.memory

        action = runner.find_action("Delay")
        if action is None:
            action = runner.find_action("Gain 2 memory")
        assert action is not None, (
            f"Should find delay action. Actions: {runner.actions()}"
        )

        runner.execute(action)
        runner.auto_resolve()

        assert game.memory == mem_before + 2, (
            f"Delay should gain 2 memory. Before: {mem_before}, after: {game.memory}"
        )
