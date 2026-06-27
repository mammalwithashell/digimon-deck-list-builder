"""Unit tests for league evaluation: matchup-matrix assembly + anchored
promotion (add-deck-specialist-league, Group 5). `play_one` is faked so no real
models/engine are needed."""
from __future__ import annotations

import json

from digimon_gym.agents import league_eval as LE
from digimon_gym.agents.anchored_eval import MatchRecord
from digimon_gym.agents.specialist_registry import Specialist, SpecialistRegistry

LAYOUT = "deadbeef"
DECKS = ["ST-1 Gaia Red", "ST-2 Cocytus Blue", "ST-4 Giga Green"]
DECK_CARDS = {d: [f"{d}-CARD"] for d in DECKS}


def _registry(tmp_path):
    reg = SpecialistRegistry(tmp_path / "r.json")
    for d in DECKS:
        reg.set_current(Specialist(
            deck=d, weights_path=f"{d}.zip", algorithm="mlp",
            observation_profile="p", tensor_layout_hash=LAYOUT, round=1))
    return reg


def test_matrix_covers_all_pairs_and_greedy(tmp_path, monkeypatch):
    reg = _registry(tmp_path)
    # Fake play_one: agent (deck1) always beats opponent unless decks identical.
    seen_decks = []

    def fake_play_one(model, opp_fn, d1, d2, seed, profile):
        seen_decks.append((d1, d2))
        return 1 if d1 != d2 else None  # win unless mirror -> draw

    monkeypatch.setattr(LE, "play_one", fake_play_one)
    matrix = LE.build_matchup_matrix(
        reg, DECK_CARDS, LAYOUT,
        model_loader=lambda wp: ("model", wp),
        policy_loader=lambda wp, algo: ("policy", wp),
        n=4, include_greedy=True,
    )
    # one row per deck; columns = greedy + every deck
    assert set(matrix) == set(DECKS)
    for ad in DECKS:
        assert set(matrix[ad]) == set(DECKS) | {"greedy"}
        # mirror cell is the draw (win_rate 0 under fake), off-diagonal is a win
        assert matrix[ad][ad]["win_rate"] == 0.0
        for od in DECKS:
            if od != ad:
                assert matrix[ad][od]["win_rate"] == 1.0
            assert matrix[ad][od]["games"] == 4
    # decks were FIXED per seat (agent deck1, opponent deck2) — not pool-sampled
    all_decks = list(DECK_CARDS.values())
    assert all(d1 in all_decks and d2 in all_decks for d1, d2 in seen_decks)


def test_matrix_excludes_layout_incompatible(tmp_path, monkeypatch):
    reg = SpecialistRegistry(tmp_path / "r.json")
    reg.set_current(Specialist(deck="ST-1 Gaia Red", weights_path="a.zip", algorithm="mlp",
                               observation_profile="p", tensor_layout_hash=LAYOUT, round=1))
    reg.set_current(Specialist(deck="ST-2 Cocytus Blue", weights_path="b.zip", algorithm="mlp",
                               observation_profile="p", tensor_layout_hash="OTHER", round=1))
    monkeypatch.setattr(LE, "play_one", lambda *a, **k: 1)
    matrix = LE.build_matchup_matrix(
        reg, {"ST-1 Gaia Red": ["x"], "ST-2 Cocytus Blue": ["y"]}, LAYOUT,
        model_loader=lambda wp: wp, policy_loader=lambda wp, a: wp, n=2)
    assert set(matrix) == {"ST-1 Gaia Red"}  # the OTHER-layout specialist is excluded
    assert "ST-2 Cocytus Blue" not in matrix["ST-1 Gaia Red"]


def test_write_matrix_roundtrip(tmp_path):
    out = LE.write_matchup_matrix(tmp_path / "m.json", {"ST-1 Gaia Red": {"greedy": {"win_rate": 0.7, "games": 10}}}, round=2)
    data = json.loads(out.read_text())
    assert data["round"] == 2 and data["matrix"]["ST-1 Gaia Red"]["greedy"]["win_rate"] == 0.7


def test_should_promote_threshold():
    assert LE.should_promote(MatchRecord(wins=14, losses=10, draws=0)) is True   # 0.583 >= 0.55
    assert LE.should_promote(MatchRecord(wins=12, losses=12, draws=0)) is False  # 0.5 < 0.55
    assert LE.should_promote(MatchRecord()) is False                             # no games
