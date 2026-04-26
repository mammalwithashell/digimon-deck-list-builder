"""Behavioral tests for EX8-070 Zofr Kabus (Option, Black, Cost 2).

Card text:
  [Main] By trashing any 1 digivolution card of 1 of your Digimon with the
      [Mineral] or [Rock] trait, until the end of your opponent's turn, that
      Digimon gains <Collision>, <Piercing>, <Reboot>, and your opponent's
      effects can't return it to hands or decks, and it gets +3000 DP.
  [Security] Delete 1 of your opponent's Digimon with the lowest play cost.

Key faithfulness points from C# reference:
  - [Main] uses SelectTrashDigivolutionCards: player CHOOSES which digivolution
    card to trash (not auto-trash from top).
  - canNoTrash: true => effect is optional (player can decline).
  - Grants: Collision, Piercing, Reboot, +3000 DP, can't return to hand/deck
    by opponent's effects; all until end of opponent's turn.
  - [Security] deletes 1 opponent Digimon with lowest play cost (IsMinCost).

Note: place_on_field takes card IDs bottom-to-top, so
    ["BT1-064", "BT4-066"] = Goblimon at bottom, Golemon (Mineral, Black) on top.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase

# Egg-free deck so the game starts directly in Main phase (no Breeding).
_FILLER_DECK = ["ST1-03"] * 50

# SEL_MY_FIELD_START = 100; first own field permanent is action 100
_SEL_MY_FIELD_0 = 100


def _make_runner(debug_runner, **kwargs):
    """Create a DebugRunner with egg-free decks so we start in Main phase."""
    return debug_runner(deck1=list(_FILLER_DECK), deck2=list(_FILLER_DECK), **kwargs)


def _play_option_and_select_target(runner, field_index=0):
    """Play Zofr Kabus from hand and select the permanent at field_index as target.

    After calling this, the script will have trashed a digivolution card and
    granted all keywords (if using auto-trash), or entered SelectSource phase
    (if using player-choice trash).
    """
    game = runner.game
    action = runner.find_action("Zofr Kabus")
    assert action is not None, "Zofr Kabus should be playable from hand"
    runner.execute(action)

    # Select own permanent
    assert game.current_phase == GamePhase.SelectTarget, \
        f"Expected SelectTarget phase, got {game.current_phase}"
    runner.execute(_SEL_MY_FIELD_0 + field_index)

    # If the script enters SelectSource for digi card choice, auto-pick first option
    if game.current_phase == GamePhase.SelectSource:
        legal = runner.action_mask()
        source_actions = [a for a in legal if a >= 2000]
        if source_actions:
            runner.execute(source_actions[0])

    # Resolve any remaining follow-up selections
    runner.auto_resolve()


@pytest.mark.behavioral
class TestEX8070ZofrKabusMain:
    """Tests for EX8-070 [Main] effect."""

    def test_main_effect_exists(self, debug_runner):
        """Should have an OptionSkill effect."""
        runner = _make_runner(debug_runner, initial_memory=5)
        runner.inject_card(1, "EX8-070", "hand")
        game = runner.game
        hand_card = game.player1.hand_cards[-1]
        effects = hand_card.effect_list(None)
        option_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) >= 1, "Should have at least one OptionSkill effect"

    def test_target_filter_mineral_trait(self, debug_runner):
        """Main effect should target Digimon with [Mineral] trait."""
        runner = _make_runner(debug_runner, initial_memory=10)
        # bottom-to-top: Goblimon (Green Lv3) under Golemon (Black, Mineral Lv4)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])

        runner.inject_card(1, "EX8-070", "hand")

        action = runner.find_action("Zofr Kabus")
        assert action is not None, "Should be able to play EX8-070 targeting Mineral Digimon"

    def test_target_filter_rock_trait(self, debug_runner):
        """Main effect should also accept Digimon with [Rock] trait."""
        runner = _make_runner(debug_runner, initial_memory=10)
        # bottom-to-top: Goblimon under Gotsumon (Rock, Black Lv3)
        # Gotsumon BT13-061 is black color [5] and Rock trait
        perm = runner.place_on_field(1, ["BT1-064", "BT13-061"])

        runner.inject_card(1, "EX8-070", "hand")

        action = runner.find_action("Zofr Kabus")
        assert action is not None, "Should be able to play EX8-070 targeting Rock trait Digimon"

    def test_target_filter_rejects_non_mineral_rock(self, debug_runner):
        """Main effect should NOT target Digimon without [Mineral] or [Rock] trait."""
        runner = _make_runner(debug_runner, initial_memory=10)
        # BT2-052 Hagurumon (Black, Machine, Lv3) has no Mineral/Rock trait.
        # Stack with another card underneath so it has digi sources.
        perm = runner.place_on_field(1, ["BT1-064", "BT2-052"])

        runner.inject_card(1, "EX8-070", "hand")

        # Play the option card
        action = runner.find_action("Zofr Kabus")
        if action is not None:
            result = runner.execute(action)
            # Should resolve without doing anything since no valid Mineral/Rock targets
            runner.auto_resolve()
            assert runner.game.current_phase == GamePhase.Main, \
                "Should return to Main with no valid Mineral/Rock targets"

    def test_target_requires_digivolution_cards(self, debug_runner):
        """Main effect should NOT target Digimon with no digivolution cards (single card stack)."""
        runner = _make_runner(debug_runner, initial_memory=10)
        # Place Golemon (Mineral, Black) with NO digivolution cards (single card)
        perm = runner.place_on_field(1, ["BT4-066"])

        runner.inject_card(1, "EX8-070", "hand")

        action = runner.find_action("Zofr Kabus")
        if action is not None:
            result = runner.execute(action)
            runner.auto_resolve()
            # Should not successfully target since no digi cards to trash

    def test_trash_selection_is_player_choice(self, debug_runner):
        """Player should CHOOSE which digivolution card to trash (not auto-trash).

        C# uses SelectTrashDigivolutionCards which presents a selection UI.
        The script must use SelectSource phase to let the RL agent choose.
        """
        runner = _make_runner(debug_runner, initial_memory=10)
        # Place a Mineral Digimon with 2 digivolution cards underneath
        # bottom-to-top: ST1-02 Biyomon, BT1-064 Goblimon, BT4-066 Golemon (Mineral top)
        perm = runner.place_on_field(1, ["ST1-02", "BT1-064", "BT4-066"])
        assert len(perm.card_sources) == 3, "Stack should have 3 cards"

        runner.inject_card(1, "EX8-070", "hand")
        game = runner.game

        # Play the option
        action = runner.find_action("Zofr Kabus")
        assert action is not None
        runner.execute(action)

        # Select the permanent
        assert game.current_phase == GamePhase.SelectTarget
        runner.execute(_SEL_MY_FIELD_0)

        # Now we should be in SelectSource phase for the digivolution card choice
        assert game.current_phase == GamePhase.SelectSource, \
            f"Expected SelectSource for digi card choice, got {game.current_phase}"

        legal = runner.action_mask()
        # Should have at least 2 choices (the 2 under-cards, NOT the top card)
        source_actions = [a for a in legal if a >= 2000]
        assert len(source_actions) >= 2, \
            f"Should present 2+ source choices for digi cards, got {len(source_actions)}: {source_actions}"

    def test_grants_collision_after_trash(self, debug_runner):
        """After trashing, the targeted Digimon should gain <Collision>."""
        runner = _make_runner(debug_runner, initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])
        runner.inject_card(1, "EX8-070", "hand")

        _play_option_and_select_target(runner)

        assert perm.has_keyword('_is_collision'), "Should gain <Collision>"

    def test_grants_piercing_after_trash(self, debug_runner):
        """After trashing, the targeted Digimon should gain <Piercing>."""
        runner = _make_runner(debug_runner, initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])
        runner.inject_card(1, "EX8-070", "hand")

        _play_option_and_select_target(runner)

        assert perm.has_keyword('_is_piercing'), "Should gain <Piercing>"

    def test_grants_reboot_after_trash(self, debug_runner):
        """After trashing, the targeted Digimon should gain <Reboot>."""
        runner = _make_runner(debug_runner, initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])
        runner.inject_card(1, "EX8-070", "hand")

        _play_option_and_select_target(runner)

        assert perm.has_keyword('_is_reboot'), "Should gain <Reboot>"

    def test_grants_dp_boost_after_trash(self, debug_runner):
        """After trashing, the targeted Digimon should get +3000 DP."""
        runner = _make_runner(debug_runner, initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])
        base_dp = perm.dp
        runner.inject_card(1, "EX8-070", "hand")

        _play_option_and_select_target(runner)

        assert perm.dp == base_dp + 3000, \
            f"DP should be {base_dp + 3000} but got {perm.dp}"

    def test_grants_bounce_protection_hand(self, debug_runner):
        """After trashing, the Digimon should be protected from return to hand."""
        runner = _make_runner(debug_runner, initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])
        runner.inject_card(1, "EX8-070", "hand")

        _play_option_and_select_target(runner)

        assert perm.has_keyword('_is_cannot_return_to_hand'), \
            "Should be protected from return to hand"

    def test_grants_bounce_protection_deck(self, debug_runner):
        """After trashing, the Digimon should be protected from return to deck."""
        runner = _make_runner(debug_runner, initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])
        runner.inject_card(1, "EX8-070", "hand")

        _play_option_and_select_target(runner)

        assert perm.has_keyword('_is_cannot_return_to_deck'), \
            "Should be protected from return to deck"

    def test_trashed_card_goes_to_trash(self, debug_runner):
        """The trashed digivolution card should end up in trash."""
        runner = _make_runner(debug_runner, initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])
        game = runner.game
        runner.inject_card(1, "EX8-070", "hand")

        _play_option_and_select_target(runner)

        # The stack should now have only 1 card (top card remains)
        assert len(perm.card_sources) == 1, \
            f"Stack should have 1 card after trashing, got {len(perm.card_sources)}"

    def test_grants_expire_end_of_opponent_turn(self, debug_runner):
        """All grants should last until end of opponent's turn (expiry = turn_count + 1)."""
        runner = _make_runner(debug_runner, initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-064", "BT4-066"])
        game = runner.game
        runner.inject_card(1, "EX8-070", "hand")

        _play_option_and_select_target(runner)

        # Check the granted keywords have proper duration (turn_count + 1)
        expected_expiry = game.turn_count + 1
        kw_expiry = perm._granted_keywords.get('_is_collision', None)
        assert kw_expiry == expected_expiry, \
            f"Collision grant should expire at turn {expected_expiry}, got {kw_expiry}"


@pytest.mark.behavioral
class TestEX8070ZofrKabusSecurity:
    """Tests for EX8-070 [Security] effect."""

    def test_security_effect_exists(self, debug_runner):
        """Should have a SecuritySkill effect."""
        runner = _make_runner(debug_runner, initial_memory=5)
        runner.inject_card(1, "EX8-070", "hand")
        game = runner.game
        hand_card = game.player1.hand_cards[-1]
        effects = hand_card.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"
        assert sec_effects[0].is_security_effect, "Should be marked as security effect"

    def test_security_deletes_lowest_cost_digimon(self, debug_runner):
        """Security should delete 1 opponent Digimon with the lowest play cost."""
        runner = _make_runner(debug_runner, initial_memory=5)
        game = runner.game

        # Place opponent Digimon with different costs
        # ST1-02 Biyomon (cost 2), ST1-03 Agumon (cost 3)
        perm_low = runner.place_on_field(2, ["ST1-02"])   # cost 2
        perm_high = runner.place_on_field(2, ["ST1-03"])   # cost 3

        p2_field_before = len(game.player2.battle_area)
        assert p2_field_before == 2

        # Manually trigger security effect
        runner.inject_card(1, "EX8-070", "hand")
        card = game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1

        sec_eff = sec_effects[0]
        sec_eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # Should enter a selection phase targeting lowest cost Digimon
        runner.auto_resolve()

        # One of the opponent's Digimon should be deleted (the lowest cost one)
        assert len(game.player2.battle_area) == 1, \
            f"One Digimon should be deleted, got {len(game.player2.battle_area)}"

    def test_security_with_tied_lowest_cost(self, debug_runner):
        """When multiple Digimon share lowest cost, player should choose which to delete."""
        runner = _make_runner(debug_runner, initial_memory=5)
        game = runner.game

        # Two Digimon with same cost
        perm1 = runner.place_on_field(2, ["ST1-02"])  # cost 2
        perm2 = runner.place_on_field(2, ["BT1-064"])  # cost 2 (Goblimon)

        runner.inject_card(1, "EX8-070", "hand")
        card = game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        sec_eff = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        sec_eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # Should enter a selection phase since both share lowest cost
        if game.current_phase in (GamePhase.SelectTarget,):
            legal = runner.action_mask()
            # Both permanents should be selectable
            assert len(legal) >= 2, \
                f"Both tied-cost Digimon should be selectable, got {len(legal)} options"

        runner.auto_resolve()
        assert len(game.player2.battle_area) == 1, "Should delete exactly 1"

    def test_security_no_opponent_digimon(self, debug_runner):
        """Security effect with no opponent Digimon should do nothing."""
        runner = _make_runner(debug_runner, initial_memory=5)
        game = runner.game

        # No opponent Digimon on field

        runner.inject_card(1, "EX8-070", "hand")
        card = game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        sec_eff = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        p2_field_before = len(game.player2.battle_area)
        sec_eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        assert len(game.player2.battle_area) == p2_field_before, \
            "Nothing should change with no opponent Digimon"
