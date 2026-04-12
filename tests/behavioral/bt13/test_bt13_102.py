"""Behavioral tests for BT13-102 Keenan Crier.

Card text:
  Tamer | Purple | Cost: 3
  [On Play] Your opponent may trash 1 Tamer card or Option card in their hand.
  If they don't, gain 1 memory and <Draw 1>.
  [Opponent's Turn] When an effect plays a Digimon, by suspending this Tamer,
  gain 1 memory.

Key faithfulness points tested:
  - Effect 0: On Play gives the *opponent* the choice to trash a Tamer/Option
    from their hand. If they decline (or have none), owner gains 1 memory and
    draws 1 card. C# order: Draw 1 then gain 1 memory.
  - Effect 1: Only triggers on opponent's turn when a Digimon is played BY EFFECT
    (not normal hand-play). Costs suspending this tamer to gain 1 memory.
  - Effect 1 must NOT trigger on normal Digimon play from hand.
"""

import pytest


KEENAN = "BT13-102"
AGUMON = "ST1-03"         # Lv.3 Digimon (filler for both decks)
GREYMON = "ST1-07"        # Lv.4 Digimon (for opponent normal play)
# We need a Tamer card and Option card to put in opponent's hand
TAI_KAMIYA = "BT1-085"    # Tamer card
GAIA_FORCE = "BT1-095"    # Option card


@pytest.mark.behavioral
class TestBT13102OnPlayDecline:
    """Effect 0: opponent declines to trash -> owner gains 1 memory + draw 1."""

    def test_opponent_declines_owner_gains_memory_and_draws(self, debug_runner):
        """When opponent has no valid cards, owner should gain 1 memory and draw 1."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[AGUMON] * 50,
            initial_memory=5,
        )
        runner.inject_card(1, KEENAN, "hand")
        # Ensure opponent hand has NO Tamers or Options (only Digimon)
        # Clear opponent hand and inject only Digimon
        runner.clear_zone(2, "hand")
        runner.inject_card(2, AGUMON, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        lib_before = snap_before.p1_library_size
        memory_before = snap_before.memory

        action = runner.find_action("Play")
        assert action is not None, f"Should find Play action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Owner should have gained 1 memory (net: spent 3 for Keenan, gained 1 back)
        expected_memory = memory_before - 3 + 1
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory} (started {memory_before}, "
            f"paid 3 cost, gained 1 from decline). Got {snap_after.memory}"
        )
        # Owner should have drawn 1 card
        assert snap_after.p1_library_size == lib_before - 1, (
            f"Library should decrease by 1 (draw). Was {lib_before}, now {snap_after.p1_library_size}"
        )

    def test_opponent_has_tamer_can_trash_it(self, debug_runner):
        """When opponent has a Tamer, they should be given the choice to trash it."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[TAI_KAMIYA] + [AGUMON] * 49,
            initial_memory=5,
        )
        runner.inject_card(1, KEENAN, "hand")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, TAI_KAMIYA, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)

        # Should be in a selection phase for the opponent
        snap = runner.snapshot()
        assert snap.phase != "Main", (
            f"Should enter selection phase for opponent. Phase: {snap.phase}"
        )
        # The active player should be player 2 (opponent making the choice)
        assert snap.active_player == 2, (
            f"Opponent (player 2) should be selecting. Active: {snap.active_player}"
        )

    def test_opponent_trashes_tamer_no_memory_gain(self, debug_runner):
        """If opponent trashes a Tamer, owner does NOT gain memory or draw."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[TAI_KAMIYA] + [AGUMON] * 49,
            initial_memory=5,
        )
        runner.inject_card(1, KEENAN, "hand")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, TAI_KAMIYA, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        lib_before = snap_before.p1_library_size

        action = runner.find_action("Play")
        runner.execute(action)

        # Opponent selects the Tamer to trash (SEL_HAND_START + 0 = action 0)
        from digimon_gym.engine.game.constants import SEL_HAND_START
        trash_action = SEL_HAND_START + 0
        legal = runner.action_mask()
        assert trash_action in legal, (
            f"Action {trash_action} should be legal for opponent to select Tamer. "
            f"Legal: {legal}"
        )
        runner.execute(trash_action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Memory should be just the cost of playing Keenan (3), no bonus
        expected_memory = snap_before.memory - 3
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory} (no gain since opponent trashed). "
            f"Got {snap_after.memory}"
        )
        # Library should NOT decrease (no draw)
        assert snap_after.p1_library_size == lib_before, (
            f"Library should not change (no draw). Was {lib_before}, now {snap_after.p1_library_size}"
        )
        # Tamer should be in opponent's trash
        assert TAI_KAMIYA in snap_after.p2_trash, (
            f"Tai Kamiya should be in opponent's trash. Trash: {snap_after.p2_trash}"
        )

    def test_opponent_has_option_can_trash_it(self, debug_runner):
        """Opponent should also be able to trash an Option card."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[GAIA_FORCE] + [AGUMON] * 49,
            initial_memory=5,
        )
        runner.inject_card(1, KEENAN, "hand")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, GAIA_FORCE, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play")
        runner.execute(action)

        # Opponent selects the Option to trash
        from digimon_gym.engine.game.constants import SEL_HAND_START
        trash_action = SEL_HAND_START + 0
        legal = runner.action_mask()
        assert trash_action in legal, (
            f"Option should be selectable. Legal: {legal}"
        )
        runner.execute(trash_action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert GAIA_FORCE in snap_after.p2_trash, (
            f"Gaia Force should be in opponent's trash. Trash: {snap_after.p2_trash}"
        )

    def test_opponent_declines_with_valid_cards(self, debug_runner):
        """Opponent may decline even when they have valid Tamer/Option cards."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[TAI_KAMIYA] + [AGUMON] * 49,
            initial_memory=5,
        )
        runner.inject_card(1, KEENAN, "hand")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, TAI_KAMIYA, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        lib_before = snap_before.p1_library_size

        action = runner.find_action("Play")
        runner.execute(action)

        # Opponent should be able to pass/decline (action 62)
        pass_action = runner.find_action("Pass")
        if pass_action is None:
            pass_action = runner.find_action("Decline")
        if pass_action is None:
            pass_action = 62  # Direct decline action
        legal = runner.action_mask()
        assert pass_action in legal, (
            f"Opponent should be able to decline. Pass action={pass_action}, legal={legal}"
        )
        runner.execute(pass_action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Owner gains memory+1 and draws 1 when opponent declines
        expected_memory = snap_before.memory - 3 + 1
        assert snap_after.memory == expected_memory, (
            f"Owner should gain 1 memory after opponent declines. "
            f"Expected {expected_memory}, got {snap_after.memory}"
        )
        assert snap_after.p1_library_size == lib_before - 1, (
            f"Owner should draw 1 after opponent declines. "
            f"Lib was {lib_before}, now {snap_after.p1_library_size}"
        )


def _simulate_effect_play(game, player, card):
    """Simulate an effect playing a card (like effect_play_from_zone does internally).

    This directly places the card on the field and fires OnEnterFieldAnyone
    with is_effect_play=True, mimicking what the engine does for effect plays.
    """
    from digimon_gym.engine.data.enums import EffectTiming
    played_perm = player.play_card_from_source(card, pay_cost=False)
    game.execute_effects(
        EffectTiming.OnEnterFieldAnyone,
        {
            "played_card": card,
            "played_permanent": played_perm,
            "event_permanent": played_perm,
            "event_player": player,
            "is_effect_play": True,
        },
    )
    return played_perm


@pytest.mark.behavioral
class TestBT13102OpponentTurnEffectPlay:
    """Effect 1: [Opponent's Turn] When an effect plays a Digimon, suspend to gain 1 memory."""

    def test_triggers_on_effect_play_digimon(self, debug_runner):
        """Should trigger when opponent plays a Digimon by effect during their turn."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[AGUMON] * 50,
            initial_memory=0,
            first_player=2,  # P2's turn (opponent of Keenan's owner)
        )
        # Place Keenan on P1's field (already active)
        runner.place_on_field(1, [KEENAN])
        runner.set_phase("Main")

        # It's P2's turn (memory is negative). Simulate an effect play.
        game = runner.game
        p2 = game.player2
        # Inject a Digimon into P2's hand to play by effect
        runner.inject_card(2, AGUMON, "hand")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        # Simulate effect play of a Digimon
        target_card = p2.hand_cards[-1]
        _simulate_effect_play(game, p2, target_card)

        # Keenan's effect should trigger (optional — auto-resolve accepts it)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Keenan should be suspended
        keenan_perm = next(
            (s for s in snap_after.p1_field if s.card_id == KEENAN), None
        )
        assert keenan_perm is not None, "Keenan should be on field"
        assert keenan_perm.is_suspended, "Keenan should be suspended after using its effect"
        # Owner (P1) gained 1 memory. Since P2 is turn player, P1 gaining memory
        # subtracts from game.memory (which tracks turn player's perspective).
        assert snap_after.memory == memory_before - 1, (
            f"P1 should gain 1 memory. Before: {memory_before}, after: {snap_after.memory}"
        )

    def test_does_not_trigger_on_normal_play(self, debug_runner):
        """Should NOT trigger when opponent plays a Digimon normally from hand."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[AGUMON] * 50,
            initial_memory=0,
            first_player=2,  # P2's turn
        )
        # Place Keenan on P1's field
        runner.place_on_field(1, [KEENAN])
        # Put a Digimon in P2's hand to play normally
        runner.inject_card(2, AGUMON, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        # Simulate a NORMAL play (not by effect) — fire OnEnterFieldAnyone
        # WITHOUT is_effect_play
        from digimon_gym.engine.data.enums import EffectTiming
        game = runner.game
        p2 = game.player2
        target_card = p2.hand_cards[-1]
        played_perm = p2.play_card_from_source(target_card, pay_cost=False)
        game.execute_effects(
            EffectTiming.OnEnterFieldAnyone,
            {
                "played_card": target_card,
                "played_permanent": played_perm,
                "event_permanent": played_perm,
                "event_player": p2,
                # NO is_effect_play — this is a normal play
            },
        )
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Keenan should NOT be suspended — this was a normal play, not an effect play
        keenan_perm = next(
            (s for s in snap_after.p1_field if s.card_id == KEENAN), None
        )
        assert keenan_perm is not None, "Keenan should still be on field"
        assert not keenan_perm.is_suspended, (
            "Keenan should NOT suspend on normal Digimon play (only effect plays)"
        )

    def test_does_not_trigger_on_own_turn(self, debug_runner):
        """Should NOT trigger during P1's own turn even if a Digimon is effect-played."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[AGUMON] * 50,
            initial_memory=5,  # P1's turn (positive memory)
        )
        runner.place_on_field(1, [KEENAN])
        runner.inject_card(1, AGUMON, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()

        # Simulate effect play during P1's own turn
        game = runner.game
        p1 = game.player1
        target_card = p1.hand_cards[-1]
        _simulate_effect_play(game, p1, target_card)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        keenan_perm = next(
            (s for s in snap_after.p1_field if s.card_id == KEENAN), None
        )
        assert keenan_perm is not None
        assert not keenan_perm.is_suspended, (
            "Keenan should NOT suspend during own turn"
        )

    def test_does_not_trigger_when_already_suspended(self, debug_runner):
        """Should NOT trigger if Keenan is already suspended."""
        runner = debug_runner(
            deck1=[KEENAN] + [AGUMON] * 49,
            deck2=[AGUMON] * 50,
            initial_memory=0,
            first_player=2,  # P2's turn
        )
        # Place Keenan already suspended
        runner.place_on_field(1, [KEENAN], is_suspended=True)
        runner.inject_card(2, AGUMON, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        # Simulate effect play
        game = runner.game
        p2 = game.player2
        target_card = p2.hand_cards[-1]
        _simulate_effect_play(game, p2, target_card)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Memory should NOT change (trigger doesn't fire)
        assert snap_after.memory == memory_before, (
            f"Memory should not change when Keenan already suspended. "
            f"Before: {memory_before}, after: {snap_after.memory}"
        )
