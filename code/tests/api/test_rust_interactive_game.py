"""Behavioral tests for the PvP interactive runner on the Rust engine.

`RustInteractiveGame` replaces the retired Python `InteractiveGame` for
WebSocket PvP (excise-legacy-engine-from-hosted-api / full cutover). These
exercise the exact surface `ws_games.py` + `ws_manager.py` consume — step,
get_action_mask, the `.game.*` shim (to_ui_json / current_player_id /
describe_actions / winner / game_over), surrender, and the event drain — plus
per-player redaction over the runner's state (rules 9 & 14).
"""

from __future__ import annotations

import pytest

pytest.importorskip("digimon_engine")

from server.rust_interactive_game import RustInteractiveGame
from server.state_filter import filter_state_for_player

_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45


def _first_legal(mask) -> int:
    for i, v in enumerate(mask.tolist()):
        if v > 0.5:
            return i
    raise AssertionError("no legal action in mask")


def _new_game(seed: int = 1) -> RustInteractiveGame:
    return RustInteractiveGame(_DECK, _DECK, seed=seed)


def test_construct_exposes_ws_surface():
    g = _new_game()
    assert g.is_game_over is False
    assert g.game.current_player_id in (1, 2)
    assert g.game.game_over is False
    assert g.game.winner is None
    assert g.game.describe_actions(1) == {}  # Rust path: no server-side descriptions
    state = g.game.to_ui_json()
    assert "player1" in state and "player2" in state and "memoryGauge" in state
    mask = g.get_action_mask()
    assert len(mask) > 0 and _first_legal(mask) >= 0


def test_step_drives_a_full_game_to_a_winner():
    g = _new_game(seed=1)
    steps = 0
    while not g.is_game_over and steps < 2000:
        g.step(_first_legal(g.get_action_mask()))
        steps += 1
    assert g.is_game_over is True
    assert g.game.game_over is True
    assert g.game.winner is not None
    assert g.game.winner.player_id in (1, 2)


def test_surrender_declares_opponent_winner():
    g = _new_game(seed=2)
    # Advance off mulligan so it's a normal mid-game concede.
    g.step(_first_legal(g.get_action_mask()))
    g.surrender(1)
    assert g.is_game_over is True
    assert g.game.winner is not None
    assert g.game.winner.player_id == 2


def test_event_drain_and_clear():
    g = _new_game(seed=3)
    g.step(_first_legal(g.get_action_mask()))
    events = g.get_last_events()
    assert isinstance(events, list)
    g.clear_events()
    assert g.get_last_events() == []
    # get_last_log is a no-op stub (matches the /games path)
    assert g.get_last_log() == []
    g.clear_log()  # must not raise


def test_redaction_over_runner_state():
    g = _new_game(seed=4)
    full = g.game.to_ui_json()
    filtered = filter_state_for_player(full, player_id=1)
    # Opponent hand/security redacted; viewer keeps own hand; counts survive.
    assert filtered["player2"]["handIds"] == []
    assert filtered["player2"]["handCards"] == []
    assert filtered["player2"]["securityIds"] == []
    assert filtered["player2"]["handCount"] == full["player2"]["handCount"]
    assert filtered["player1"]["handIds"] == full["player1"]["handIds"]
    assert filtered["player1"]["securityIds"] == []


def test_ignores_legacy_interactive_game_kwargs():
    # lobby historically passed player1_type/player2_type/agent_action_delay_ms;
    # the Rust runner must accept and ignore them (human-vs-human, no bot policy).
    g = RustInteractiveGame(
        _DECK, _DECK, player1_type="Human", player2_type="Human", agent_action_delay_ms=0
    )
    assert g.is_game_over is False
