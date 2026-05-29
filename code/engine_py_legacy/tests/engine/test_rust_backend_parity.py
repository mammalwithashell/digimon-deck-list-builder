"""Parity: Rust `RustHeadlessGame` matches Python `HeadlessGame` at the
mask/tensor level for a fixed seed + deck pair.

Skipped automatically when the Rust wheel is not installed, so CI without
the binding can still run this file. Install the binding with:

    cd digimon-engine-py && maturin develop --release
"""
from __future__ import annotations

import numpy as np
import pytest

from engine_py_legacy.engine.runners.headless_game import HeadlessGame

pytest.importorskip("digimon_engine")
from digimon_engine import RustHeadlessGame  # noqa: E402


# Deck used by DigimonEnv's default configuration — small, deterministic.
DECK1 = ["ST1-01"] * 5 + ["ST1-03"] * 45
DECK2 = ["ST1-01"] * 5 + ["ST1-03"] * 45

# This file compares the sunset Python `HeadlessGame` (v1-shaped tensors)
# against the Rust `RustHeadlessGame`. After `flip-engine-default-to-lite-
# deck-v2`, the Rust default is `standard_lite_deck_v2` (8850), so we must
# pin the Rust side to `standard_compact_v1` (1375) for shape parity to be
# meaningful. The test exists to detect drift in the v1 layout — not to
# claim the two engines share a default.
_PARITY_PROFILE = "standard_compact_v1"


def _build():
    py = HeadlessGame(DECK1, DECK2)
    rs = RustHeadlessGame(DECK1, DECK2, observation_profile=_PARITY_PROFILE)
    return py, rs


def test_action_mask_shape_and_dtype_match():
    py, rs = _build()
    py_mask = py.get_action_mask()
    rs_mask = rs.get_action_mask()
    assert py_mask.shape == rs_mask.shape
    assert py_mask.dtype == np.float32
    assert rs_mask.dtype == np.float32


def test_board_tensor_shape_and_dtype_match():
    py, rs = _build()
    py_obs = py.get_board_tensor(1)
    rs_obs = rs.get_board_tensor(1)
    assert py_obs.shape == rs_obs.shape
    assert py_obs.dtype == np.float32
    assert rs_obs.dtype == np.float32


def test_is_game_over_initial_state():
    py, rs = _build()
    assert py.is_game_over == rs.is_game_over is False


def test_pass_everything_terminates_rust_backend():
    """Default policy (pass every step) must terminate the game in the Rust
    backend within a generous turn cap, matching the Python side."""
    # Profile pinned for shape consistency with the rest of the file — game
    # termination itself is profile-agnostic.
    rs = RustHeadlessGame(DECK1, DECK2, observation_profile=_PARITY_PROFILE)
    winner = rs.run_until_conclusion(max_turns=2000)
    assert rs.is_game_over is True
    assert winner in (1, 2)


def test_env_swap_via_backend_env_var(monkeypatch):
    """DigimonEnv picks the Rust backend when DIGIMON_BACKEND=rust is set."""
    from digimon_gym.digimon_gym import DigimonEnv

    monkeypatch.setenv("DIGIMON_BACKEND", "rust")
    env = DigimonEnv()
    obs, info = env.reset()
    assert obs.shape[0] > 0
    assert "action_mask" in info
    assert info["action_mask"].shape == obs.shape[:0] + (2192,)
    # Confirm we really got the Rust runner.
    assert type(env.runner).__name__ == "RustHeadlessGame"


# ── New parity tests for WS-bound surfaces (Task 6) ──────────────────────────


def test_to_ui_json_top_level_keys_match():
    py, rs = _build()
    from engine_py_legacy.engine.game.serialization import to_ui_json
    py_ui = to_ui_json(py.game)
    rs_ui = rs.to_ui_json()
    assert set(rs_ui.keys()) == set(py_ui.keys()), (
        f"UI-JSON key set differs.\nRust only: {set(rs_ui) - set(py_ui)}\n"
        f"Python only: {set(py_ui) - set(rs_ui)}"
    )


def test_to_ui_json_player_ids_use_python_convention():
    _, rs = _build()
    ui = rs.to_ui_json()
    assert ui["player1"]["id"] == 1
    assert ui["player2"]["id"] == 2
    assert ui["currentPlayer"] in (1, 2)


def test_player_ui_data_key_set_matches():
    py, rs = _build()
    from engine_py_legacy.engine.game.serialization import to_ui_json
    py_ui = to_ui_json(py.game)
    rs_ui = rs.to_ui_json()
    assert set(rs_ui["player1"].keys()) == set(py_ui["player1"].keys()), (
        f"player1 key set differs.\nRust only: "
        f"{set(rs_ui['player1']) - set(py_ui['player1'])}\n"
        f"Python only: {set(py_ui['player1']) - set(rs_ui['player1'])}"
    )


def test_pending_selection_returns_none_at_start():
    _, rs = _build()
    assert rs.get_pending_selection() is None


def test_get_events_returns_list():
    _, rs = _build()
    rs.step(62)  # PASS
    events = rs.get_events_since_last_step()
    assert isinstance(events, list)
    for ev in events:
        assert "type" in ev
        assert "seq" in ev
        assert "player" in ev
        assert "meta" in ev


def test_events_drained_between_calls():
    _, rs = _build()
    rs.step(62)
    _ = rs.get_events_since_last_step()
    # No step between calls → no new events
    assert rs.get_events_since_last_step() == []


def test_recording_returns_none_without_record_flag():
    rs = RustHeadlessGame(DECK1, DECK2, observation_profile=_PARITY_PROFILE)
    assert rs.get_recording() is None


def test_recording_has_expected_shape():
    rs = RustHeadlessGame(
        DECK1, DECK2, record_actions=True, observation_profile=_PARITY_PROFILE
    )
    rs.step(0)
    rs.step(0)
    rec = rs.get_recording()
    assert rec is not None
    for key in (
        "initial_state",
        "actions",
        "total_actions",
        "tensor_snapshots_count",
        # NOTE: "tensor_snapshots" is omitted when empty (record_tensors=False).
        # Check only tensor_snapshots_count; the key itself is conditional.
    ):
        assert key in rec, f"missing {key} in recording"
    assert rec["total_actions"] == 2
    assert rec["initial_state"]["player1"]["player_id"] == 1
    assert rec["initial_state"]["player2"]["player_id"] == 2
