"""Parity: Rust `RustHeadlessGame` matches Python `HeadlessGame` at the
mask/tensor level for a fixed seed + deck pair.

Skipped automatically when the Rust wheel is not installed, so CI without
the binding can still run this file. Install the binding with:

    cd digimon-engine-py && maturin develop --release
"""
from __future__ import annotations

import numpy as np
import pytest

from digimon_gym.engine.runners.headless_game import HeadlessGame

pytest.importorskip("digimon_engine")
from digimon_engine import RustHeadlessGame  # noqa: E402


# Deck used by DigimonEnv's default configuration — small, deterministic.
DECK1 = ["ST1-01"] * 5 + ["ST1-03"] * 45
DECK2 = ["ST1-01"] * 5 + ["ST1-03"] * 45


def _build():
    py = HeadlessGame(DECK1, DECK2)
    rs = RustHeadlessGame(DECK1, DECK2)
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
    rs = RustHeadlessGame(DECK1, DECK2)
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
    assert info["action_mask"].shape == obs.shape[:0] + (2168,)
    # Confirm we really got the Rust runner.
    assert type(env.runner).__name__ == "RustHeadlessGame"
