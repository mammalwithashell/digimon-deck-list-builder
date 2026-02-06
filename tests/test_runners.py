"""Tests for HeadlessGame and InteractiveGame runners."""

import os
import sys
import pytest
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.data.enums import GamePhase, PlayerType
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.game import ACTION_SPACE_SIZE, TENSOR_SIZE


# ─── Helpers ─────────────────────────────────────────────────────────

def make_test_deck():
    """Return a valid deck of card IDs for testing."""
    return ["ST1-01"] * 5 + ["ST1-03"] * 45


@pytest.fixture(autouse=True)
def reset_registry():
    """Reset CardRegistry before each test to ensure isolation."""
    CardRegistry.reset()
    yield
    CardRegistry.reset()


# ─── HeadlessGame Tests ─────────────────────────────────────────────

class TestHeadlessGame:
    def test_construction(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        assert not game.is_game_over
        assert game.winner_id is None

    def test_starts_at_breeding(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        assert game.game.current_phase == GamePhase.Breeding

    def test_action_mask_shape(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        mask = game.get_action_mask()
        assert mask.shape == (ACTION_SPACE_SIZE,)
        assert mask.dtype == np.float32

    def test_board_tensor_shape(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        tensor = game.get_board_tensor()
        assert tensor.shape == (TENSOR_SIZE,)
        assert tensor.dtype == np.float32

    def test_board_tensor_player_specific(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        t1 = game.get_board_tensor(1)
        t2 = game.get_board_tensor(2)
        # Memory should be negated between perspectives
        assert t1[2] == -t2[2]

    def test_step_breeding_pass(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        assert game.game.current_phase == GamePhase.Breeding
        game.step(62)  # pass breeding
        assert game.game.current_phase == GamePhase.Main

    def test_step_hatch(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        # Should be in breeding phase
        assert game.game.current_phase == GamePhase.Breeding
        # Check that hatch is valid (action 60)
        mask = game.get_action_mask()
        if mask[60] > 0.5:
            game.step(60)  # hatch
            assert game.game.turn_player.breeding_area is not None

    def test_step_pass_turn(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        # Pass breeding
        game.step(62)
        assert game.game.current_phase == GamePhase.Main
        # Pass turn
        game.step(62)
        # Should have advanced to next turn's breeding
        assert game.game.current_phase == GamePhase.Breeding
        assert game.game.turn_count == 2

    def test_step_on_game_over_is_noop(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        game.game.declare_winner(game.game.player1)
        assert game.is_game_over
        phase_before = game.game.current_phase
        game.step(62)  # should be a no-op
        assert game.game.current_phase == phase_before

    def test_run_until_conclusion_default_policy(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        winner = game.run_until_conclusion(max_turns=50)
        assert winner in (0, 1, 2)
        assert game.is_game_over

    def test_run_until_conclusion_with_policy(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)

        def simple_policy(g, mask):
            """Play first valid action, prefer non-pass."""
            valid = np.where(mask > 0.5)[0]
            for a in valid:
                if a != 62:
                    return int(a)
            return 62

        winner = game.run_until_conclusion(max_turns=200, policy_fn=simple_policy)
        assert winner in (0, 1, 2)
        assert game.is_game_over

    def test_verbose_mode_captures_logs(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, verbose=True)
        game.step(62)  # breeding pass
        game.step(62)  # pass turn
        logs = game.get_last_log()
        assert len(logs) > 0

    def test_silent_mode_no_logs(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, verbose=False)
        game.step(62)
        game.step(62)
        logs = game.get_last_log()
        assert len(logs) == 0

    def test_mask_valid_actions_in_breeding(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        mask = game.get_action_mask()
        # Pass (62) should always be valid in breeding
        assert mask[62] == 1.0
        # Hatch (60) should be valid if digitama deck has cards
        if len(game.game.turn_player.digitama_library_cards) > 0:
            assert mask[60] == 1.0

    def test_winner_id_after_conclusion(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        game.run_until_conclusion(max_turns=20)
        assert game.winner_id in (1, 2)


# ─── InteractiveGame Tests ──────────────────────────────────────────

class TestInteractiveGame:
    def test_construction_human_vs_agent(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        assert not game.is_game_over
        assert game.player1_type == PlayerType.Human
        assert game.player2_type == PlayerType.Agent

    def test_construction_all_combos(self):
        deck = make_test_deck()
        for p1 in PlayerType:
            for p2 in PlayerType:
                game = InteractiveGame(deck, deck, p1, p2)
                assert game.player1_type == p1
                assert game.player2_type == p2

    def test_is_current_player_human(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        # Turn player is randomly assigned, check consistency
        if game.game.turn_player is game.game.player1:
            assert game.is_current_player_human() is True
        else:
            assert game.is_current_player_human() is False

    def test_run_step_pauses_on_human(self):
        deck = make_test_deck()
        # Force player1 to be human and go first
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        # Ensure P1 goes first
        game.game.turn_player = game.game.player1
        game.game.opponent_player = game.game.player2

        state = game.run_step()
        # Should return state without advancing (paused for human)
        assert "CurrentPhase" in state
        assert game.is_current_player_human() is True

    def test_run_step_agent_auto_plays(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Agent, PlayerType.Agent)
        phase_before = game.game.current_phase
        state = game.run_step()
        # Agent should have taken an action (default: pass)
        assert "CurrentPhase" in state

    def test_step_executes_action(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Human)
        assert game.game.current_phase == GamePhase.Breeding
        game.step(62)  # pass breeding
        assert game.game.current_phase == GamePhase.Main

    def test_get_state_returns_dict(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        state = game.get_state()
        assert isinstance(state, dict)
        assert "TurnCount" in state
        assert "CurrentPhase" in state
        assert "IsGameOver" in state
        assert "Player1" in state
        assert "Player2" in state

    def test_get_action_mask(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        mask = game.get_action_mask()
        assert mask.shape == (ACTION_SPACE_SIZE,)

    def test_log_capture_and_clear(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Human)
        game.step(62)  # breeding pass
        logs = game.get_last_log()
        assert len(logs) > 0  # VerboseLogger should have captured something
        game.clear_log()
        assert len(game.get_last_log()) == 0

    def test_human_vs_human_full_turn(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Human)

        # P1 breeding pass
        game.step(62)
        assert game.game.current_phase == GamePhase.Main

        # P1 pass turn
        game.step(62)
        assert game.game.current_phase == GamePhase.Breeding
        assert game.game.turn_count == 2

        # P2 breeding pass
        game.step(62)
        assert game.game.current_phase == GamePhase.Main

        # P2 pass turn
        game.step(62)
        assert game.game.current_phase == GamePhase.Breeding
        assert game.game.turn_count == 3

    def test_game_over_returns_state(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        game.game.declare_winner(game.game.player1)
        state = game.run_step()
        assert state["IsGameOver"] is True
        assert state["Winner"] == game.game.player1.player_id

    def test_step_on_game_over_is_noop(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        game.game.declare_winner(game.game.player1)
        phase_before = game.game.current_phase
        game.step(62)
        assert game.game.current_phase == phase_before


# ─── Integration: to_json() Tests ────────────────────────────────────

class TestGameToJson:
    def test_to_json_structure(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        state = game.game.to_json()

        assert "TurnCount" in state
        assert "CurrentPhase" in state
        assert "CurrentPlayer" in state
        assert "MemoryGauge" in state
        assert "IsGameOver" in state
        assert "Winner" in state
        assert "Player1" in state
        assert "Player2" in state

    def test_to_json_player_data(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        state = game.game.to_json()

        for key in ["Player1", "Player2"]:
            p = state[key]
            assert "Id" in p
            assert "Memory" in p
            assert "HandCount" in p
            assert "HandIds" in p
            assert "SecurityCount" in p
            assert "DeckCount" in p
            assert "BattleAreaCount" in p
            assert "BattleArea" in p

    def test_to_json_initial_state(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        state = game.game.to_json()

        assert state["TurnCount"] == 1
        assert state["IsGameOver"] is False
        assert state["Winner"] is None
        # Each player should have 5 hand cards and 5 security after setup
        for key in ["Player1", "Player2"]:
            p = state[key]
            assert p["HandCount"] == 5
            assert p["SecurityCount"] == 5
