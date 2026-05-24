"""Smoke tests for the PyO3-exposed concede and play-order primitives used
by best-of-three match training.

The full engine-level behavior is covered by
`code/digimon-engine/tests/concede_primitive.rs` and `select_play_order.rs`.
This module pins the PyO3 binding contract: Python 1/2 player IDs translate
correctly to Rust 0/1, the `win_reason` field appears on the terminal
outcome dict, and the play-order accessors round-trip cleanly.
"""

from __future__ import annotations

import pytest

pytest.importorskip("digimon_engine")

from digimon_engine import RustHeadlessGame  # noqa: E402


def _starter_decks() -> tuple[list[str], list[str]]:
    deck = ["ST1-01"] * 5 + ["ST1-03"] * 45
    return deck, deck


def test_concede_by_player_one_declares_player_two_winner() -> None:
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=7)

    # Conceder uses Python 1/2 convention. Pre-concede the game is live.
    assert not game.is_game_over

    winner = game.concede(1)
    assert game.is_game_over
    assert winner == 2, "opponent of conceder (Python pid 2) is the winner"
    assert game.winner_id == 2


def test_concede_by_player_two_declares_player_one_winner() -> None:
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=7)

    winner = game.concede(2)
    assert winner == 1
    assert game.winner_id == 1


def test_concede_surfaces_win_reason_in_terminal_outcome() -> None:
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=7)

    game.concede(1)
    outcome = game.get_terminal_outcome()

    assert outcome["game_over"] is True
    assert outcome["winner_id"] == 2
    assert outcome["result"] == "win"
    assert outcome["reason"] == "concede"
    # `win_reason` is the BO3 match-training alias for `reason` — recordings
    # consume this field name directly.
    assert outcome["win_reason"] == "concede"


def test_concede_via_action_93_routes_through_decode() -> None:
    """Action 93 in the standard action interface MUST trigger concede."""
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=7)

    # Drive past the mulligan phase so the agent has decisions.
    while game.mulligan_current_player is not None:
        game.accept_mulligan(game.mulligan_current_player, True)

    decision_player = game.current_player_id
    game.step(93)

    assert game.is_game_over
    outcome = game.get_terminal_outcome()
    assert outcome["win_reason"] == "concede"
    # Decision player conceded; opponent wins.
    assert outcome["winner_id"] != decision_player


def test_concede_after_game_over_is_idempotent() -> None:
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=7)

    game.concede(1)
    outcome_first = game.get_terminal_outcome()
    # Second concede call must not change the winner or reason.
    game.concede(2)
    outcome_second = game.get_terminal_outcome()

    assert outcome_first == outcome_second


def test_request_play_order_selection_installs_phase() -> None:
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=7)

    # Drive past mulligan so the engine isn't in an exclusive phase.
    while game.mulligan_current_player is not None:
        game.accept_mulligan(game.mulligan_current_player, True)

    # Wrapper calls this between games; Python 1/2 pid.
    game.request_play_order_selection(2)
    # No choice taken yet.
    assert game.take_play_order_choice() is None

    # Action 94 = play first.
    game.step(94)
    chosen = game.take_play_order_choice()
    assert chosen == "first"
    # Once read, the slot resets — second take returns None.
    assert game.take_play_order_choice() is None


def test_request_play_order_selection_play_second() -> None:
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=7)
    while game.mulligan_current_player is not None:
        game.accept_mulligan(game.mulligan_current_player, True)

    game.request_play_order_selection(1)
    game.step(95)
    assert game.take_play_order_choice() == "second"


def test_concede_invalid_pid_raises() -> None:
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=7)
    with pytest.raises((ValueError, RuntimeError)):
        game.concede(0)  # Python convention is 1 or 2
    with pytest.raises((ValueError, RuntimeError)):
        game.concede(3)
