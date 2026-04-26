"""Behavioral tests for BT22-004 Wanyamon (Lv.2 DigiEgg, Green, [Lesser, CS]).

Card text:
  Inherited Effect [Your Turn] [Once Per Turn] When effects place [CS] trait
  Digimon cards in this Digimon's digivolution cards, it may digivolve into
  a [CS] trait Digimon card in the hand with the digivolution cost reduced
  by 1.

Key verification:
  - Inherited effect: is_inherited_effect=True (DigiEgg)
  - Trigger on OnAddDigivolutionCards timing
  - Condition: added_card has [CS] trait AND the event permanent is
    Wanyamon's permanent (not some other permanent on field)
  - Condition: [Your Turn] gate (card.owner.is_my_turn)
  - Action: effect_digivolve_from_hand with CS Digimon filter and
    cost_reduction=1
  - Optional (the "may digivolve" wording)
  - Once per turn
  - Only fires when Wanyamon is in a stack on the field/breeding (inherited)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# BT22-004 Wanyamon: Lv.2 DigiEgg, Green, [Lesser, CS]
WANYAMON = "BT22-004"

# ST4-03 Tentomon: Lv.3 Green, no CS trait (used as base for Wanyamon stack)
BASE_GREEN_L3 = "ST4-03"

# BT22-044 Palmon: Lv.3 Green [Plant, CS] — a CS Digimon (cost 3, evo cost 1)
CS_DIGI_L3_GREEN = "BT22-044"

# BT22-046 Gargomon: Lv.4 Green [Beast, CS] — Lv4 target for digivolve (evo cost 3)
CS_DIGI_L4_GREEN = "BT22-046"

# BT1-067 Palmon: Lv.3 Green, no CS trait — non-CS card for negative cases
NON_CS_DIGI_L3_GREEN = "BT1-067"


def _get_inherited_effects(perm):
    """Return effects sourced from non-top card sources (inherited slots)."""
    effects = []
    for source in perm.card_sources[:-1]:
        for eff in source.effect_list(EffectTiming.NoTiming):
            if eff.is_inherited_effect:
                effects.append(eff)
    return effects


def _find_wanyamon_trigger(perm):
    """Find the OnAddDigivolutionCards digivolve-from-hand inherited effect."""
    for eff in _get_inherited_effects(perm):
        if eff.timing == EffectTiming.OnAddDigivolutionCards:
            return eff
    return None


def _find_wanyamon_card_source(perm):
    """Return the Wanyamon CardSource inside perm's stack (if any)."""
    for cs in perm.card_sources:
        if cs.c_entity_base and cs.c_entity_base.card_id == WANYAMON:
            return cs
    return None


@pytest.mark.behavioral
class TestBT22004Structure:
    """Static structure of the effect surface."""

    def test_wanyamon_has_inherited_ondigi_trigger(self, debug_runner):
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        trig = _find_wanyamon_trigger(perm)
        assert trig is not None, (
            "Wanyamon should expose an inherited effect bound to "
            "OnAddDigivolutionCards"
        )
        assert trig.is_inherited_effect is True
        assert trig.is_optional is True, (
            "'it may digivolve' must be optional so the RL agent can decline"
        )
        # Once per turn
        assert trig.can_activate_this_turn()
        trig.record_activation()
        assert not trig.can_activate_this_turn(), (
            "Trigger must be OPT — second activation in same turn blocked"
        )

    def test_no_active_trigger_from_hand(self, debug_runner):
        """Inherited effect should NOT surface from the hand zone."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, WANYAMON, "hand")
        # Inherited effects on a hand card don't fire as active triggers.
        # Just verify condition fails when Wanyamon has no permanent_of_this_card.
        wany = runner.game.player1.hand_cards[-1]
        # Build an effect instance from the script for direct condition check
        effs = wany.effect_list(EffectTiming.OnAddDigivolutionCards)
        assert len(effs) >= 1
        trig = effs[0]
        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert trig.can_use_condition(ctx) is False, (
            "Condition must fail when Wanyamon is not on field (no perm)"
        )


@pytest.mark.behavioral
class TestBT22004Condition:
    """Condition gates for the OnAddDigivolutionCards trigger."""

    def _setup(self, debug_runner, memory=10):
        runner = debug_runner(initial_memory=memory)
        perm = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        # Force turn player=1 (is_my_turn = True for player1)
        game = runner.game
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        return runner, perm

    def test_condition_true_when_cs_card_added_to_this_perm(self, debug_runner):
        runner, perm = self._setup(debug_runner)
        game = runner.game
        trig = _find_wanyamon_trigger(perm)
        assert trig is not None

        # Fake an "added_card" = a CS Digimon card source
        cs_card = runner.inject_card(1, CS_DIGI_L3_GREEN, "hand")
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm, 'event_permanent': perm,
            'added_card': cs_card,
        }
        assert trig.can_use_condition(ctx) is True, (
            "Should trigger when a CS Digimon is placed into THIS perm's stack"
        )

    def test_condition_false_when_non_cs_card_added(self, debug_runner):
        runner, perm = self._setup(debug_runner)
        game = runner.game
        trig = _find_wanyamon_trigger(perm)

        non_cs = runner.inject_card(1, NON_CS_DIGI_L3_GREEN, "hand")
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm, 'event_permanent': perm,
            'added_card': non_cs,
        }
        assert trig.can_use_condition(ctx) is False, (
            "Must NOT trigger when the placed card lacks [CS] trait"
        )

    def test_condition_false_when_card_added_to_different_perm(self, debug_runner):
        """If a CS card is placed into a DIFFERENT permanent's stack,
        Wanyamon (under perm A) must not fire."""
        runner, perm_a = self._setup(debug_runner)
        game = runner.game
        # Another permanent (no Wanyamon)
        perm_b = runner.place_on_field(1, [BASE_GREEN_L3])
        trig = _find_wanyamon_trigger(perm_a)

        cs_card = runner.inject_card(1, CS_DIGI_L3_GREEN, "hand")
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm_a,   # Wanyamon's perm (effect source)
            'event_permanent': perm_b,  # Card was added to different perm
            'added_card': cs_card,
        }
        assert trig.can_use_condition(ctx) is False, (
            "Must NOT trigger when the card was added to a different permanent"
        )

    def test_condition_false_on_opponent_turn(self, debug_runner):
        """[Your Turn] gate — must not trigger when it's the opponent's turn."""
        runner, perm = self._setup(debug_runner)
        game = runner.game
        # Flip turn to opponent
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        trig = _find_wanyamon_trigger(perm)

        cs_card = runner.inject_card(1, CS_DIGI_L3_GREEN, "hand")
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm, 'event_permanent': perm,
            'added_card': cs_card,
        }
        assert trig.can_use_condition(ctx) is False, (
            "[Your Turn] gate must block on opponent's turn"
        )

    def test_condition_false_without_added_card_context(self, debug_runner):
        """If context has no added_card, condition should not fire."""
        runner, perm = self._setup(debug_runner)
        game = runner.game
        trig = _find_wanyamon_trigger(perm)
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm, 'event_permanent': perm,
        }
        assert trig.can_use_condition(ctx) is False, (
            "Must not fire when there is no added_card payload"
        )


@pytest.mark.behavioral
class TestBT22004Process:
    """Process (effect resolution): request digivolve from hand, CS filter, -1 cost."""

    def test_process_opens_hand_selection_for_cs_digimon(self, debug_runner):
        """When fired, the process should open a SelectTarget selection
        offering the CS Digimon from hand."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        # Put Wanyamon under the base Green Lv3 so it becomes the permanent
        perm = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # Put a CS Lv3 Digimon in hand (can digivolve from Lv3 base into Lv3? NO —
        # we want to digivolve the Lv3 top card into a Lv4 CS). Use Gargomon L4.
        cs_hand = runner.inject_card(1, CS_DIGI_L4_GREEN, "hand")
        # Also put a non-CS card in hand that should NOT appear as a valid option
        non_cs_hand = runner.inject_card(1, NON_CS_DIGI_L3_GREEN, "hand")

        trig = _find_wanyamon_trigger(perm)
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm, 'event_permanent': perm,
            'added_card': cs_hand,  # dummy CS card for condition check
        }

        # Invoke process directly
        trig.on_process_callback(ctx)

        # A selection phase should be active
        assert game.pending_selection is not None, (
            "Process should open a selection for digivolve from hand"
        )
        valid = game.pending_selection.valid_indices
        # Verify that the valid indices point to Gargomon in hand (and NOT the non-CS)
        from engine_py_legacy.engine.game.constants import SEL_HAND_START
        valid_cards = []
        for idx in valid:
            if idx >= SEL_HAND_START:
                hand_i = idx - SEL_HAND_START
                if 0 <= hand_i < len(game.player1.hand_cards):
                    valid_cards.append(game.player1.hand_cards[hand_i].card_id)
        assert CS_DIGI_L4_GREEN in valid_cards, (
            f"Gargomon (CS Lv4) must be a valid digivolve-from-hand choice. "
            f"Got: {valid_cards}"
        )
        assert NON_CS_DIGI_L3_GREEN not in valid_cards, (
            "Non-CS hand cards must not appear as valid choices"
        )

    def test_process_is_optional(self, debug_runner):
        """The selection should be flagged optional (may digivolve)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        perm = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        cs_hand = runner.inject_card(1, CS_DIGI_L4_GREEN, "hand")

        trig = _find_wanyamon_trigger(perm)
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm, 'event_permanent': perm,
            'added_card': cs_hand,
        }
        trig.on_process_callback(ctx)

        assert game.pending_selection is not None
        assert game.pending_selection.is_optional is True, (
            "Selection must be optional ('may digivolve')"
        )

    def test_process_applies_cost_reduction_of_1(self, debug_runner):
        """When the agent picks a CS card from hand, the memory cost paid
        must be (base digivolve cost - 1), floor 0."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        perm = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # Gargomon BT22-046 is Lv4 Green CS, play_cost=5, evo cost = 3
        # digivolve_from_hand uses get_cost_itself (play_cost) minus reduction.
        # With cost_reduction=1, cost = max(0, 5 - 1) = 4.
        cs_hand = runner.inject_card(1, CS_DIGI_L4_GREEN, "hand")

        # Clear hand to leave only Gargomon (simpler valid_indices)
        game.player1.hand_cards[:] = [cs_hand]

        mem_before = game.memory
        trig = _find_wanyamon_trigger(perm)
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm, 'event_permanent': perm,
            'added_card': cs_hand,
        }
        trig.on_process_callback(ctx)

        # Pick the only valid hand index
        assert game.pending_selection is not None
        valid = game.pending_selection.valid_indices
        assert len(valid) >= 1
        runner.execute(valid[0])

        # Gargomon should now be the top of the permanent stack
        assert perm.top_card is cs_hand, (
            "Gargomon (CS) should be the new top card after digivolve"
        )
        # Memory: Gargomon play_cost=5, reduced by 1 -> 4
        # lose_memory(4) in DCGO equals: memory -= 4
        # (Exact memory math depends on runner's convention — compare to expected.)
        mem_after = game.memory
        assert mem_before - mem_after == 4, (
            f"Should lose 4 memory (5 base - 1 reduction). "
            f"Before: {mem_before}, after: {mem_after}"
        )

    def test_process_non_cs_hand_card_ignored(self, debug_runner):
        """If the ONLY hand card is non-CS, the effect should have no valid
        target — selection should either not open or immediately be empty."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        perm = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        non_cs = runner.inject_card(1, NON_CS_DIGI_L3_GREEN, "hand")
        game.player1.hand_cards[:] = [non_cs]

        trig = _find_wanyamon_trigger(perm)
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': perm, 'event_permanent': perm,
            'added_card': non_cs,  # the process doesn't re-check this
        }
        # Call process — since only non-CS in hand, effect_digivolve_from_hand
        # should early-return without opening a selection.
        trig.on_process_callback(ctx)
        assert game.pending_selection is None, (
            "No valid CS target -> selection should not open"
        )


@pytest.mark.behavioral
class TestBT22004EndToEnd:
    """Integration via add_card_source to verify the full trigger chain."""

    def test_trigger_fires_on_real_add_card_source(self, debug_runner):
        """When a CS card is actually added to Wanyamon's stack,
        execute_effects should surface the Wanyamon trigger."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        perm = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # Gargomon in hand as CS Lv4 digivolve target
        cs_hand = runner.inject_card(1, CS_DIGI_L4_GREEN, "hand")
        game.player1.hand_cards[:] = [cs_hand]

        # Simulate an "effect places a CS Digimon card in this Digimon's stack".
        # Create a CS card source (e.g. another CS Lv3 card) and add it to perm.
        from engine_py_legacy.engine.debug.state_injection import _create_card
        cs_placed = _create_card(CS_DIGI_L3_GREEN, game.player1)
        perm.add_card_source(cs_placed)

        # After the synchronous fire, Wanyamon's OPT should have queued
        # a selection for the optional digivolve.
        # Auto-resolve should then take the first valid action (digivolve).
        if game.pending_selection is None:
            # Nothing queued — BUG (no trigger)
            pytest.fail(
                "Expected Wanyamon trigger to open a selection after "
                "adding a CS card to its permanent's stack"
            )
        assert game.pending_selection.is_optional is True

    def test_trigger_does_not_fire_for_other_perm(self, debug_runner):
        """CS card added to a DIFFERENT permanent must not trigger Wanyamon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        perm_a = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        perm_b = runner.place_on_field(1, [BASE_GREEN_L3])
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        from engine_py_legacy.engine.debug.state_injection import _create_card
        cs_placed = _create_card(CS_DIGI_L3_GREEN, game.player1)
        perm_b.add_card_source(cs_placed)

        assert game.pending_selection is None, (
            "Wanyamon under perm_a must NOT fire when card is placed under perm_b"
        )

    def test_opt_blocks_second_placement(self, debug_runner):
        """After firing once in a turn, a second CS placement must not retrigger."""
        runner = debug_runner(initial_memory=20)
        game = runner.game
        perm = runner.place_on_field(1, [WANYAMON, BASE_GREEN_L3])
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # First placement
        trig = _find_wanyamon_trigger(perm)
        assert trig is not None
        trig.record_activation()  # Simulate it already fired this turn

        from engine_py_legacy.engine.debug.state_injection import _create_card
        cs_placed = _create_card(CS_DIGI_L3_GREEN, game.player1)
        perm.add_card_source(cs_placed)

        # OPT guard in execute_effects should have skipped it
        assert game.pending_selection is None, (
            "OPT must block second activation in the same turn"
        )
