"""Behavioral tests for BT24-008 Elizamon (Lv.3 Red Digimon).

Card text:
[On Play] By trashing 1 card with the [Reptile], [Dragonkin] or [LIBERATOR]
trait from your hand, <Draw 2> (Draw 2 cards from your deck.)

Inherited Effect:
[Your Turn] [Once Per Turn] When your opponent's security stack is removed
from, gain 1 memory.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase

# Minimal decks for testing (need at least 50 cards for game start)
DECK_WITH_BT24_008 = ["BT24-008"] * 4 + ["ST1-03"] * 46
FILLER_DECK = ["ST1-03"] * 50


@pytest.mark.behavioral
class TestBT24008OnPlay:
    """Tests for BT24-008 [On Play] effect: trash qualifying card -> Draw 2."""

    def test_on_play_trash_reptile_draw_2(self, debug_runner):
        """Playing Elizamon with a Reptile card in hand should allow trashing
        it and drawing 2 cards."""
        # ST1-03 Agumon has Reptile trait; use as trash fodder
        runner = debug_runner(
            deck1=DECK_WITH_BT24_008,
            deck2=FILLER_DECK,
            initial_memory=5,
            starting_hand1=["BT24-008", "ST1-03"],
        )
        game = runner.game
        deck_before = len(game.player1.library_cards)

        # Play Elizamon
        play_action = runner.find_action("Play Elizamon")
        assert play_action is not None, "Should be able to play Elizamon"
        runner.execute(play_action)

        # Should enter SelectHand phase for trash selection
        assert game.current_phase == GamePhase.SelectHand, \
            f"Should be in SelectHand phase, got {game.current_phase}"

        # Select the qualifying hand card (first valid action in the mask)
        legal = runner.action_mask()
        hand_select = [a for a in legal if a != 62]
        assert len(hand_select) > 0, "Should have at least 1 hand card to select"
        runner.execute(hand_select[0])
        runner.auto_resolve()

        # Verify: 1 card trashed (ST1-03), 2 cards drawn
        # Started with 2 in hand, played 1 (BT24-008), trashed 1 (ST1-03), drew 2
        # Net hand: 2 - 1 - 1 + 2 = 2
        assert len(game.player1.hand_cards) == 2, \
            f"Expected 2 cards in hand, got {len(game.player1.hand_cards)}"

        # ST1-03 should be in trash
        trash_ids = [c.c_entity_base.card_id for c in game.player1.trash_cards]
        assert "ST1-03" in trash_ids, "ST1-03 should be in trash"

        # Deck should have lost 2 cards (drawn)
        assert len(game.player1.library_cards) == deck_before - 2, \
            "Should have drawn 2 from deck"

    def test_on_play_decline_trash_no_draw(self, debug_runner):
        """If player declines to trash (passes optional selection), no draw
        should occur. Trash is a cost for Draw 2."""
        runner = debug_runner(
            deck1=DECK_WITH_BT24_008,
            deck2=FILLER_DECK,
            initial_memory=5,
            starting_hand1=["BT24-008", "ST1-03"],
        )
        game = runner.game
        deck_before = len(game.player1.library_cards)

        # Play Elizamon
        play_action = runner.find_action("Play Elizamon")
        assert play_action is not None
        runner.execute(play_action)

        # Decline the optional trash (action 62 = pass)
        assert game.current_phase == GamePhase.SelectHand
        runner.execute(62)  # pass/decline
        runner.auto_resolve()

        # No card should be trashed
        trash_ids = [c.c_entity_base.card_id for c in game.player1.trash_cards]
        assert "ST1-03" not in trash_ids, "ST1-03 should not be trashed when declined"

        # No draw should have happened
        assert len(game.player1.library_cards) == deck_before, \
            "Deck should be unchanged when trash is declined (no draw)"

    def test_on_play_no_qualifying_hand_cards(self, debug_runner):
        """If no qualifying cards in hand, effect should not trigger
        (no selection phase). C# checks HasMatchConditionOwnersHand."""
        # ST1-07 Greymon has Dinosaur trait (not Reptile/Dragonkin/LIBERATOR)
        runner = debug_runner(
            deck1=["BT24-008"] * 4 + ["ST1-07"] * 46,
            deck2=FILLER_DECK,
            initial_memory=5,
            starting_hand1=["BT24-008", "ST1-07"],
        )
        game = runner.game
        deck_before = len(game.player1.library_cards)

        # Play Elizamon
        play_action = runner.find_action("Play Elizamon")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve()

        # Should NOT have entered selection and should NOT have drawn
        assert game.current_phase == GamePhase.Main, \
            f"Should return to Main phase, got {game.current_phase}"
        assert len(game.player1.library_cards) == deck_before, \
            "No draw should occur when no qualifying cards in hand"

    def test_on_play_liberator_trait_qualifies(self, debug_runner):
        """Cards with LIBERATOR trait should also qualify for trash cost.
        BT24-008 itself has [Reptile, LIBERATOR]."""
        # Use another BT24-008 as the trash target (it has LIBERATOR)
        runner = debug_runner(
            deck1=["BT24-008"] * 10 + ["ST1-03"] * 40,
            deck2=FILLER_DECK,
            initial_memory=5,
            starting_hand1=["BT24-008", "BT24-008"],
        )
        game = runner.game
        deck_before = len(game.player1.library_cards)

        # Play one copy of BT24-008
        play_action = runner.find_action("Play Elizamon")
        assert play_action is not None
        runner.execute(play_action)

        # Select the other BT24-008 from hand to trash
        assert game.current_phase == GamePhase.SelectHand
        legal = runner.action_mask()
        hand_select = [a for a in legal if a != 62]
        assert len(hand_select) > 0, "Should have a card to select"
        runner.execute(hand_select[0])
        runner.auto_resolve()

        # Should have drawn 2
        assert len(game.player1.library_cards) == deck_before - 2, \
            "Should draw 2 after trashing LIBERATOR card"


@pytest.mark.behavioral
class TestBT24008Inherited:
    """Tests for BT24-008 inherited [Your Turn] [OPT] gain 1 memory
    when opponent's security is removed."""

    def test_inherited_gain_memory_on_opponent_security_loss(self, debug_runner):
        """When opponent's security is removed during your turn, gain 1 memory."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        # Place a Lv.4 Digimon with BT24-008 as inherited source
        perm = runner.place_on_field(1, ["BT24-008", "ST1-07"])

        memory_before = game.memory

        # Directly invoke the inherited effect to verify it fires correctly
        card = perm.card_sources[0]  # BT24-008 (bottom of stack)
        effects = card.effect_list(None)
        inherited_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert len(inherited_effects) == 1, "Should have exactly 1 OnLoseSecurity inherited effect"

        eff = inherited_effects[0]
        assert eff.is_inherited_effect, "Must be inherited"

        # Simulate context: opponent (player2) lost security during player1's turn
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # opponent lost security
        }
        assert eff.can_use_condition(context), \
            "Condition should pass: it's our turn and opponent lost security"

        eff.on_process_callback(context)
        assert game.memory == memory_before + 1, \
            f"Should gain 1 memory, was {memory_before}, now {game.memory}"

    def test_inherited_no_trigger_on_own_security_loss(self, debug_runner):
        """When YOUR OWN security is removed, the inherited effect should NOT
        trigger. Card text says 'opponent's security stack'."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        perm = runner.place_on_field(1, ["BT24-008", "ST1-07"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Simulate context: player1 (self) lost security during player1's turn
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,  # OWN security loss
        }
        assert not eff.can_use_condition(context), \
            "Condition should FAIL when own security is lost"

    def test_inherited_no_trigger_on_opponents_turn(self, debug_runner):
        """The inherited effect is [Your Turn] — should not trigger on
        the opponent's turn."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        perm = runner.place_on_field(1, ["BT24-008", "ST1-07"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Simulate opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # opponent lost security
        }
        assert not eff.can_use_condition(context), \
            "Condition should FAIL when it's not our turn"

    def test_inherited_once_per_turn(self, debug_runner):
        """The effect is [Once Per Turn] — second activation should be blocked."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        perm = runner.place_on_field(1, ["BT24-008", "ST1-07"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        eff = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,
        }

        # First activation
        assert eff.can_activate_this_turn(), "First activation should be allowed"
        eff.record_activation()
        eff.on_process_callback(context)

        memory_after_first = game.memory

        # Second activation should be blocked by OPT
        assert not eff.can_activate_this_turn(), \
            "Second activation should be blocked by Once Per Turn"
