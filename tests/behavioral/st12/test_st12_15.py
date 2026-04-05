"""Behavioral tests for ST12-15 From Master to Disciple (Option, Red, Cost 2).

Card text:
[Main] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] or
[Sistermon] in its name or [Royal Knight] trait among them to your hand.
Trash the rest. Then, place this card in your battle area.
[Main] <Delay> (By trashing this card after the placing turn, activate the
effect below.)
- The next time one of your Digimon would digivolve this turn, reduce the
  digivolution cost by 1.

[Security] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon]
or [Sistermon] in its name or [Royal Knight] trait among them to your hand.
Trash the rest. Then, place this card in your battle area.

Key cards used in tests:
- ST12-04 Huckmon (Lv3, Red, "Mini Dragon") - name match for [Huckmon]
- ST12-06 BaoHuckmon (Lv4, Red, "Dinosaur") - name match for [Huckmon]
- ST12-10 Jesmon (Lv6, Red, "Holy Warrior"/"Royal Knight") - trait match
- ST12-12 Sistermon Blanc (Lv3, Yellow, "Puppet") - name match for [Sistermon]
- ST12-05 Meramon (Lv4, Red, "Flame") - NO match (no Huckmon/Sistermon name, no Royal Knight trait)
- ST12-02 Candlemon (Lv3, Red, "Flame") - NO match
"""

import pytest


# Cards with matching names
HUCKMON = "ST12-04"          # Huckmon (name match)
BAOHUCKMON = "ST12-06"       # BaoHuckmon (name match)
SISTERMON_BLANC = "ST12-12"  # Sistermon Blanc (name match)

# Card with matching trait
JESMON = "ST12-10"           # Jesmon (Royal Knight trait)

# Non-matching cards
MERAMON = "ST12-05"          # Meramon (no match)
CANDLEMON = "ST12-02"       # Candlemon (no match)

# The card under test
FROM_MASTER = "ST12-15"

# Filler for decks (Red Digimon, needed for option color requirement)
FILLER = "ST1-03"  # Agumon, generic Red Lv3


def _build_deck(extra_cards=None):
    """Build a minimal 50-card deck."""
    deck = []
    if extra_cards:
        deck.extend(extra_cards)
    while len(deck) < 50:
        deck.append(FILLER)
    return deck


def _find_play_option(runner):
    """Find the action to play ST12-15 from hand."""
    action = runner.find_action("From Master to Disciple")
    if action is None:
        action = runner.find_action("ST12-15")
    return action


@pytest.mark.behavioral
class TestST12_15_RevealFilter:
    """Test that the reveal filter correctly identifies matching cards."""

    def test_huckmon_name_matches(self, debug_runner):
        """A card with 'Huckmon' in its name should be selectable from reveal."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=5)
        game = runner.game
        player = game.player1

        # Need a Red Digimon on field for option color requirement
        runner.place_on_field(1, [FILLER])

        # Set up hand and library
        runner.clear_zone(1, "hand")
        runner.inject_card(1, FROM_MASTER, "hand")
        # Stack top 3: MERAMON, CANDLEMON, HUCKMON
        runner.inject_card(1, HUCKMON, "library_top")
        runner.inject_card(1, CANDLEMON, "library_top")
        runner.inject_card(1, MERAMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert HUCKMON in hand_ids, (
            f"Huckmon should be added to hand from reveal. Hand: {hand_ids}"
        )

    def test_sistermon_name_matches(self, debug_runner):
        """A card with 'Sistermon' in its name should be selectable from reveal."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=5)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [FILLER])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, FROM_MASTER, "hand")
        runner.inject_card(1, SISTERMON_BLANC, "library_top")
        runner.inject_card(1, CANDLEMON, "library_top")
        runner.inject_card(1, MERAMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert SISTERMON_BLANC in hand_ids, (
            f"Sistermon Blanc should be added to hand from reveal. Hand: {hand_ids}"
        )

    def test_royal_knight_trait_matches(self, debug_runner):
        """A card with [Royal Knight] trait should be selectable from reveal."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=5)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [FILLER])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, FROM_MASTER, "hand")
        runner.inject_card(1, JESMON, "library_top")
        runner.inject_card(1, CANDLEMON, "library_top")
        runner.inject_card(1, MERAMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert JESMON in hand_ids, (
            f"Jesmon (Royal Knight trait) should be added to hand from reveal. Hand: {hand_ids}"
        )

    def test_non_matching_cards_trashed(self, debug_runner):
        """Non-matching revealed cards should be trashed (not returned to deck bottom)."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=5)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [FILLER])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, FROM_MASTER, "hand")
        runner.inject_card(1, HUCKMON, "library_top")
        runner.inject_card(1, CANDLEMON, "library_top")
        runner.inject_card(1, MERAMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        trash_ids = [c.card_id for c in player.trash_cards]
        # Meramon and Candlemon should be trashed (the 2 non-matching cards)
        assert MERAMON in trash_ids, (
            f"Non-matching Meramon should be trashed. Trash: {trash_ids}"
        )
        assert CANDLEMON in trash_ids, (
            f"Non-matching Candlemon should be trashed. Trash: {trash_ids}"
        )


@pytest.mark.behavioral
class TestST12_15_PlaceInBattleArea:
    """Test that the option card is placed in the battle area after use."""

    def test_main_places_in_battle_area(self, debug_runner):
        """[Main] should place this card in battle area (delay placement)."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=5)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [FILLER])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, FROM_MASTER, "hand")
        runner.inject_card(1, HUCKMON, "library_top")
        runner.inject_card(1, MERAMON, "library_top")
        runner.inject_card(1, CANDLEMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # ST12-15 should be in battle area
        ba_ids = [
            p.top_card.card_id for p in player.battle_area
            if p.top_card
        ]
        assert FROM_MASTER in ba_ids, (
            f"ST12-15 should be placed in battle area. BA: {ba_ids}"
        )


@pytest.mark.behavioral
class TestST12_15_Delay:
    """Test the Delay effect: evo cost -1 for next digivolve this turn."""

    def test_delay_available_after_placing_turn(self, debug_runner):
        """Delay should be available on a later turn (not the placing turn)."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=5)
        game = runner.game

        # Place ST12-15 in battle area as if played on turn 1
        runner.place_on_field(1, [FROM_MASTER], turn_played=1)
        game.turn_count = 2  # Now it's a later turn
        runner.set_phase("Main")

        action = runner.find_action("Delay")
        if action is None:
            action = runner.find_action("ST12-15")
        assert action is not None, (
            f"Delay should be available after the placing turn. Actions: {runner.actions()}"
        )

    def test_delay_not_available_on_placing_turn(self, debug_runner):
        """Delay should NOT be available on the turn the card was placed."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=5)
        game = runner.game

        # Place ST12-15 in battle area on the current turn
        runner.place_on_field(1, [FROM_MASTER], turn_played=1)
        game.turn_count = 1  # Same turn as placing
        runner.set_phase("Main")

        delay_actions = runner.find_actions("Delay")
        # Filter to only ST12-15 delay actions
        st12_delays = {aid: desc for aid, desc in delay_actions.items()
                       if "ST12-15" in desc or "cost" in desc.lower() or "digivol" in desc.lower()}
        assert len(st12_delays) == 0, (
            f"Delay should NOT be available on the placing turn. Found: {st12_delays}"
        )

    def test_delay_reduces_digivolution_cost(self, debug_runner):
        """Activating delay should reduce the next digivolution cost by 1."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=5)
        game = runner.game
        player = game.player1

        # Place a Lv3 Huckmon on field (for digivolving into BaoHuckmon)
        runner.place_on_field(1, [HUCKMON], turn_played=0)

        # Place ST12-15 in battle area as if played last turn
        runner.place_on_field(1, [FROM_MASTER], turn_played=0)

        # Put BaoHuckmon in hand for digivolving
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BAOHUCKMON, "hand")

        game.turn_count = 2
        runner.set_phase("Main")
        runner.set_memory(5)

        # Activate the delay
        delay_action = runner.find_action("Delay")
        if delay_action is None:
            delay_action = runner.find_action("Digivolution Cost")
        assert delay_action is not None, (
            f"Should find delay action. Actions: {runner.actions()}"
        )
        runner.execute(delay_action)
        runner.auto_resolve()

        mem_after_delay = game.memory

        # Now digivolve Huckmon (Lv3) into BaoHuckmon (Lv4)
        # BaoHuckmon (ST12-06) costs 2 to digivolve from Red Lv3
        # With delay's -1, it should cost 1
        digi_action = runner.find_action("Digivolve")
        if digi_action is None:
            digi_action = runner.find_action("BaoHuckmon")
        assert digi_action is not None, (
            f"Should find digivolve action. Actions: {runner.actions()}"
        )

        runner.execute(digi_action)
        runner.auto_resolve()

        mem_after_digi = game.memory
        actual_cost = mem_after_delay - mem_after_digi

        # Normal evo cost is 2, with -1 reduction = 1
        assert actual_cost == 1, (
            f"Digivolution should cost 1 (2 base - 1 delay reduction). "
            f"Memory before digi: {mem_after_delay}, after: {mem_after_digi}, "
            f"actual cost: {actual_cost}"
        )

    def test_delay_one_shot_only(self, debug_runner):
        """Delay evo cost -1 should only apply to the NEXT digivolve (one-shot)."""
        runner = debug_runner(deck1=_build_deck(), deck2=_build_deck(), initial_memory=10)
        game = runner.game
        player = game.player1

        # Place two Lv3 Huckmon on field
        runner.place_on_field(1, [HUCKMON], turn_played=0)
        runner.place_on_field(1, [HUCKMON], turn_played=0)

        # Place ST12-15 in battle area
        runner.place_on_field(1, [FROM_MASTER], turn_played=0)

        # Put 2 BaoHuckmon in hand for digivolving
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BAOHUCKMON, "hand")
        runner.inject_card(1, BAOHUCKMON, "hand")

        game.turn_count = 2
        runner.set_phase("Main")
        runner.set_memory(10)

        # Activate delay
        delay_action = runner.find_action("Delay")
        if delay_action is None:
            delay_action = runner.find_action("Digivolution Cost")
        assert delay_action is not None, (
            f"Should find delay action. Actions: {runner.actions()}"
        )
        runner.execute(delay_action)
        runner.auto_resolve()

        mem_after_delay = game.memory

        # First digivolve (should cost 2 - 1 = 1)
        digi_action = runner.find_action("Digivolve")
        if digi_action is None:
            digi_action = runner.find_action("BaoHuckmon")
        assert digi_action is not None
        runner.execute(digi_action)
        runner.auto_resolve()

        mem_after_first = game.memory
        first_cost = mem_after_delay - mem_after_first

        # Second digivolve (should cost normal 2, no reduction)
        digi_action2 = runner.find_action("Digivolve")
        if digi_action2 is None:
            digi_action2 = runner.find_action("BaoHuckmon")

        if digi_action2 is not None:
            runner.execute(digi_action2)
            runner.auto_resolve()
            mem_after_second = game.memory
            second_cost = mem_after_first - mem_after_second
            assert second_cost == 2, (
                f"Second digivolve should cost full 2 (one-shot reduction consumed). "
                f"Got: {second_cost}"
            )

        assert first_cost == 1, (
            f"First digivolve should cost 1 (2 - 1 reduction). Got: {first_cost}"
        )
