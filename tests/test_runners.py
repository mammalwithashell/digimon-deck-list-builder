"""Tests for HeadlessGame and InteractiveGame runners."""

import os
import sys
import pytest
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.digimon_gym import greedy_policy
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.data.enums import GamePhase, PlayerType, CardKind, CardColor
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.evo_cost import EvoCost
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.game import ACTION_SPACE_SIZE, TENSOR_SIZE
from digimon_gym.engine.game import Game


# ─── Helpers ─────────────────────────────────────────────────────────

def make_test_deck():
    """Return a valid deck of card IDs for testing."""
    return ["ST1-01"] * 5 + ["ST1-03"] * 45


def resolve_opening_mulligan(runner) -> None:
    """Advance through opening mulligan decisions by keeping hand."""
    guard = 0
    while runner.game.current_phase == GamePhase.Mulligan and guard < 4:
        mask = runner.get_action_mask()
        valid = np.where(mask > 0.5)[0]
        action = int(valid[0]) if len(valid) else 0
        runner.step(action)
        guard += 1


def make_card(
    card_id: str,
    name: str,
    owner,
    *,
    kind: CardKind = CardKind.Digimon,
    level: int = 3,
    dp: int = 3000,
    play_cost: int = 3,
    colors=None,
    evo_costs=None,
):
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.level = level
    entity.dp = dp
    entity.play_cost = play_cost
    entity.card_colors = colors or [CardColor.Red]
    if evo_costs is not None:
        entity.evo_costs = evo_costs
    card = CardSource()
    card.set_base_data(entity, owner)
    return card


def setup_policy_game(phase: GamePhase, memory: int = 3) -> Game:
    game = Game()
    game.current_phase = phase
    game.memory = memory
    game.turn_count = 2
    game.turn_player = game.player1
    game.opponent_player = game.player2
    game.player1.is_my_turn = True
    game.player2.is_my_turn = False
    return game


class PolicyEnv:
    def __init__(self, game: Game):
        self.game = game

    def get_action_mask(self):
        return np.array(
            self.game.get_action_mask(self.game.current_player_id),
            dtype=np.float32,
        )


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

    def test_starts_at_mulligan(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        assert game.game.current_phase == GamePhase.Mulligan

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
        resolve_opening_mulligan(game)
        assert game.game.current_phase == GamePhase.Breeding
        game.step(62)  # pass breeding
        assert game.game.current_phase == GamePhase.Main

    def test_step_hatch(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        resolve_opening_mulligan(game)
        # Should be in breeding phase
        assert game.game.current_phase == GamePhase.Breeding
        # Check that hatch is valid (action 60)
        mask = game.get_action_mask()
        if mask[60] > 0.5:
            game.step(60)  # hatch
            assert game.game.turn_player.breeding_area is not None
            assert game.game.current_phase == GamePhase.Main

    def test_step_pass_turn(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        resolve_opening_mulligan(game)
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
        resolve_opening_mulligan(game)
        game.step(62)  # breeding pass
        game.step(62)  # pass turn
        logs = game.get_last_log()
        assert len(logs) > 0

    def test_silent_mode_no_logs(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, verbose=False)
        resolve_opening_mulligan(game)
        game.step(62)
        game.step(62)
        logs = game.get_last_log()
        assert len(logs) == 0

    def test_mask_valid_actions_in_breeding(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        resolve_opening_mulligan(game)
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

class TestGreedyPolicy:
    def test_mulligan_prefers_redraw_without_level3(self):
        game = setup_policy_game(GamePhase.Mulligan, memory=0)
        p1 = game.player1
        game.active_player = p1
        game._mulligan_order = [game.player1, game.player2]
        game._mulligan_used = {1: False, 2: False}

        p1.hand_cards.extend([
            make_card("A-001", "Lv4", p1, level=4),
            make_card("A-002", "Lv5", p1, level=5),
            make_card("A-003", "Lv6", p1, level=6),
            make_card("A-004", "Tamer", p1, kind=CardKind.Tamer, level=0, dp=0),
            make_card("A-005", "Option", p1, kind=CardKind.Option, level=0, dp=0),
        ])

        action = greedy_policy(PolicyEnv(game))
        assert action == 1

    def test_prefers_keep_turn_digivolve_over_play(self):
        game = setup_policy_game(GamePhase.Main, memory=3)
        p1 = game.player1

        base = make_card("BASE-001", "Base", p1, level=3, dp=3000)
        p1.battle_area.append(Permanent([base]))

        evo = make_card(
            "EVO-001",
            "Evo",
            p1,
            level=4,
            dp=6000,
            play_cost=4,
            evo_costs=[EvoCost(card_color=CardColor.Red, level=3, memory_cost=2)],
        )
        p1.hand_cards.append(evo)

        action = greedy_policy(PolicyEnv(game))
        assert action == 400  # Digivolve hand[0] -> field[0]

    def test_prefers_keep_turn_digivolve_over_pass_turn_digivolve(self):
        game = setup_policy_game(GamePhase.Main, memory=3)
        p1 = game.player1

        base_a = make_card("BASE-001", "BaseA", p1, level=3, dp=3000)
        base_b = make_card("BASE-002", "BaseB", p1, level=3, dp=3000)
        p1.battle_area.extend([Permanent([base_a]), Permanent([base_b])])

        keep_turn_evo = make_card(
            "EVO-KEEP",
            "KeepTurnEvo",
            p1,
            level=4,
            dp=5000,
            play_cost=4,
            evo_costs=[EvoCost(card_color=CardColor.Red, level=3, memory_cost=2)],
        )
        pass_turn_evo = make_card(
            "EVO-PASS",
            "PassTurnEvo",
            p1,
            level=5,
            dp=7000,
            play_cost=7,
            evo_costs=[EvoCost(card_color=CardColor.Red, level=3, memory_cost=4)],
        )
        p1.hand_cards.extend([keep_turn_evo, pass_turn_evo])

        action = greedy_policy(PolicyEnv(game))
        assert action == 400  # Keep-turn option hand[0] -> field[0]

    def test_no_keep_turn_digivolve_attacks_before_play(self):
        game = setup_policy_game(GamePhase.Main, memory=1)
        p1 = game.player1
        p2 = game.player2

        base = make_card("BASE-001", "AttackerBase", p1, level=3, dp=4000)
        p1.battle_area.append(Permanent([base]))

        evo = make_card(
            "EVO-001",
            "PassTurnEvoOnly",
            p1,
            level=4,
            dp=6000,
            play_cost=4,
            evo_costs=[EvoCost(card_color=CardColor.Red, level=3, memory_cost=2)],
        )
        play_card = make_card(
            "PLAY-001",
            "Playable",
            p1,
            kind=CardKind.Tamer,
            level=0,
            dp=0,
            play_cost=1,
            colors=[CardColor.Red],
        )
        p1.hand_cards.extend([evo, play_card])

        # Prevent lethal categorization while keeping security attack legal.
        p2.security_cards.append(make_card("SEC-001", "Sec", p2))

        action = greedy_policy(PolicyEnv(game))
        assert action == 112  # Attack with slot 0 at security target 12

    def test_breeding_prefers_hatch_when_available(self):
        game = setup_policy_game(GamePhase.Breeding, memory=0)
        p1 = game.player1

        egg = make_card(
            "EGG-001",
            "Digitama",
            p1,
            kind=CardKind.DigiEgg,
            level=2,
            dp=0,
            play_cost=0,
            colors=[CardColor.Red],
        )
        p1.digitama_library_cards.append(egg)

        action = greedy_policy(PolicyEnv(game))
        assert action == 60


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
        # Should return UI state without advancing (paused for human)
        assert "currentPhase" in state
        assert game.is_current_player_human() is True

    def test_run_step_agent_auto_plays(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Agent, PlayerType.Agent)
        phase_before = game.game.current_phase
        state = game.run_step()
        # Agent should have taken an action (default: pass)
        assert "currentPhase" in state

    def test_run_step_applies_agent_action_delay(self, monkeypatch):
        deck = make_test_deck()
        game = InteractiveGame(
            deck,
            deck,
            PlayerType.Agent,
            PlayerType.Agent,
            agent_action_delay_ms=250,
        )

        sleep_calls = []
        monkeypatch.setattr(
            "digimon_gym.engine.runners.interactive_game.time.sleep",
            lambda seconds: sleep_calls.append(seconds),
        )

        game.run_step()
        assert sleep_calls == [0.25]

    def test_run_step_advances_until_human_turn(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent, player2_policy="greedy")
        resolve_opening_mulligan(game)

        # Force agent to act with only pass-like progression available.
        game.game.turn_player = game.game.player2
        game.game.opponent_player = game.game.player1
        game.game.player1.is_my_turn = False
        game.game.player2.is_my_turn = True
        game.game.current_phase = GamePhase.Main
        game.game.memory = -1
        game.game.player2.hand_cards = []
        game.game.player2.battle_area = []

        state = game.run_step()
        assert "currentPhase" in state
        assert game.is_current_player_human() is True

    def test_step_executes_action(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Human)
        resolve_opening_mulligan(game)
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
        resolve_opening_mulligan(game)
        game.step(62)  # breeding pass
        logs = game.get_last_log()
        assert len(logs) > 0  # VerboseLogger should have captured something
        game.clear_log()
        assert len(game.get_last_log()) == 0

    def test_human_vs_human_full_turn(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Human)
        resolve_opening_mulligan(game)

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
        assert state["isGameOver"] is True
        assert state["winner"] == game.game.player1.player_id

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
        # At mulligan: 5 hand cards, security not set yet.
        for key in ["Player1", "Player2"]:
            p = state[key]
            assert p["HandCount"] == 5
            assert p["SecurityCount"] == 0
