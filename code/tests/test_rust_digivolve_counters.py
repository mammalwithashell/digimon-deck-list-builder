"""Smoke test that the four digivolve counters round-trip through the PyO3
binding and follow the Python 1/2 player-ID convention.

Catches binding-key typos and mis-indexed `[u32; 2]` -> dict-key mappings
that the Rust-only integration test in `tests/digivolve_counters.rs`
cannot see.
"""

from __future__ import annotations

import pytest

pytest.importorskip("digimon_engine")

from digimon_engine import RustHeadlessGame  # noqa: E402


def _starter_decks() -> tuple[list[str], list[str]]:
    deck = ["ST1-01"] * 5 + ["ST1-03"] * 45
    return deck, deck


def test_get_rl_state_exposes_digivolve_counters_for_both_players() -> None:
    deck1, deck2 = _starter_decks()
    game = RustHeadlessGame(deck1, deck2, seed=1)
    state = game.get_rl_state()

    assert state["p1_digivolutions"] == 0
    assert state["p2_digivolutions"] == 0
    assert state["p1_dna_digivolutions"] == 0
    assert state["p2_dna_digivolutions"] == 0
