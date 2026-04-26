"""Behavioral tests for BT21-001 Gigimon (Lv.2 Digitama).

Card text:
Inherited Effect [Your Turn] [Once Per Turn] When your opponent's security
stack is removed from, 1 of your Digimon may digivolve into a Digimon card
with the [Reptile] or [Dragonkin] trait in the hand with the digivolution
cost reduced by 1.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT21001Gigimon:
    """Tests for BT21-001 Gigimon inherited effect."""

    def test_condition_passes_when_opponent_loses_security(self, debug_runner):
        """The inherited effect condition should pass when the opponent loses
        security during the card owner's turn."""
        runner = debug_runner(initial_memory=5)

        # Place a Digimon with BT21-001 Gigimon as digivolution source
        # BT21-008 Elizamon is a Lv.3 Reptile LIBERATOR
        perm = runner.place_on_field(1, ["BT21-001", "BT21-008"])

        game = runner.game

        # Find the inherited OnLoseSecurity effect
        card = perm.card_sources[0]  # Gigimon (bottom of stack)
        effects = card.effect_list(None)
        ols_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert len(ols_effects) == 1, "Gigimon should have exactly 1 OnLoseSecurity inherited effect"

        ols = ols_effects[0]
        assert ols.is_inherited_effect, "Effect must be inherited"
        assert ols.is_optional, "Effect must be optional"

        # Simulate context: opponent (player2) loses security, effect owner is player1
        # In the engine, OnLoseSecurity fires with extra_context {"player": <loser>}
        # which becomes context["event_player"] in the condition
        context_opponent_loses = {
            'game': game,
            'player': game.player1,        # effect owner
            'event_player': game.player2,   # the one who lost security (opponent)
            'permanent': perm,
        }
        assert ols.can_use_condition(context_opponent_loses), (
            "Condition should pass when OPPONENT loses security on owner's turn"
        )

    def test_condition_fails_when_own_security_lost(self, debug_runner):
        """The inherited effect condition must NOT pass when the card owner's
        own security is removed -- only the opponent's security removal triggers it."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT21-001", "BT21-008"])

        game = runner.game

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Simulate context: owner (player1) loses security -- should NOT trigger
        context_self_loses = {
            'game': game,
            'player': game.player1,        # effect owner
            'event_player': game.player1,   # the one who lost security (self!)
            'permanent': perm,
        }
        assert not ols.can_use_condition(context_self_loses), (
            "Condition must FAIL when the card OWNER loses security (not opponent)"
        )

    def test_condition_fails_on_opponent_turn(self, debug_runner):
        """The effect has [Your Turn] restriction -- must not trigger during
        opponent's turn even if opponent loses security."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT21-001", "BT21-008"])

        game = runner.game

        # Switch to opponent's turn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Even though opponent (player2) loses security, it's not our turn
        context = {
            'game': game,
            'player': game.player1,
            'event_player': game.player2,
            'permanent': perm,
        }
        assert not ols.can_use_condition(context), (
            "Condition must FAIL on opponent's turn ([Your Turn] restriction)"
        )

    def test_once_per_turn(self, debug_runner):
        """The effect is [Once Per Turn] -- after one activation, it should
        not activate again on the same turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT21-001", "BT21-008"])

        game = runner.game

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Verify max_count_per_turn is 1
        assert ols.max_count_per_turn == 1, "Effect should be once per turn"

        # First activation should be allowed
        assert ols.can_activate_this_turn(), "First activation should be allowed"

        # Record one activation
        ols.record_activation()

        # Second activation should be blocked
        assert not ols.can_activate_this_turn(), (
            "Second activation must be blocked (Once Per Turn)"
        )

    def test_process_selects_digimon_then_digivolves_from_hand(self, debug_runner):
        """The process callback should select a Digimon, then offer Reptile/Dragonkin
        cards from hand to digivolve into with cost -1."""
        runner = debug_runner(initial_memory=10)

        # Place BT21-001 under BT21-008 Elizamon (Lv.3 Reptile)
        perm = runner.place_on_field(1, ["BT21-001", "BT21-008"])

        # Put a Reptile Lv.4 in hand for digivolve target
        runner.inject_card(1, "BT21-017", zone="hand")  # Dimetromon Lv.4 Reptile

        game = runner.game

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Verify the process callback exists
        assert ols.on_process_callback is not None, "Process callback must be set"

        # We verify the callback calls effect_select_own_permanent
        # (Full integration would require action resolution; here we verify the
        # callback doesn't crash and makes the right engine calls)
        calls = []
        original_select = game.effect_select_own_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=True, prompt=""):
            calls.append({
                'player': player,
                'callback': callback,
                'filter_fn': filter_fn,
                'is_optional': is_optional,
            })

        game.effect_select_own_permanent = mock_select

        ols.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(calls) == 1, "Process should call effect_select_own_permanent once"
        assert calls[0]['player'] is game.player1
        assert calls[0]['is_optional'] is True

        # Verify filter only accepts Digimon
        filter_fn = calls[0]['filter_fn']
        assert filter_fn(perm), "Filter should accept a Digimon permanent"

        game.effect_select_own_permanent = original_select

    def test_digi_filter_accepts_reptile_rejects_non_reptile(self, debug_runner):
        """The digivolve filter should accept cards with [Reptile] or [Dragonkin]
        traits and reject others."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["BT21-001", "BT21-008"])

        game = runner.game

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Capture the digi_filter from the callback
        digi_filter_ref = []
        original_select = game.effect_select_own_permanent
        original_digi = game.effect_digivolve_from_hand

        def mock_select(player, callback, filter_fn=None, is_optional=True, prompt=""):
            # Call the callback with the perm to trigger effect_digivolve_from_hand
            callback(perm)

        def mock_digi(player, target_perm, digi_filter, cost_reduction=0,
                      is_optional=True, prompt="", **kwargs):
            digi_filter_ref.append(digi_filter)

        game.effect_select_own_permanent = mock_select
        game.effect_digivolve_from_hand = mock_digi

        ols.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(digi_filter_ref) == 1, "Should have captured the digi_filter"
        digi_filter = digi_filter_ref[0]

        # Create test card sources
        # CardSource.card_traits is a property returning c_entity_base.type_eng
        from digimon_gym.engine.core.card_source import CardSource
        from digimon_gym.engine.core.entity_base import CEntity_Base

        def _make_card_with_traits(traits):
            e = CEntity_Base()
            e.type_eng = traits
            cs = CardSource()
            cs.c_entity_base = e
            return cs

        reptile_card = _make_card_with_traits(["Reptile"])
        dragonkin_card = _make_card_with_traits(["Dragonkin"])
        dinosaur_card = _make_card_with_traits(["Dinosaur"])
        dragon_card = _make_card_with_traits(["Dragon"])
        no_trait_card = _make_card_with_traits([])

        assert digi_filter(reptile_card), "Should accept [Reptile] trait"
        assert digi_filter(dragonkin_card), "Should accept [Dragonkin] trait"
        assert not digi_filter(dinosaur_card), "Should reject [Dinosaur] trait"
        assert not digi_filter(dragon_card), "Should reject [Dragon] trait"
        assert not digi_filter(no_trait_card), "Should reject cards with no matching trait"

        game.effect_select_own_permanent = original_select
        game.effect_digivolve_from_hand = original_digi
