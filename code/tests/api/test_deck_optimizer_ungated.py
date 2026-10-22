"""The two-layer gate is a TRAINING gate: the hosted deck tools stay ungated.

Re-review N1 / controller ruling 2026-10-21. `/deck-optimizer/evaluate` builds its
opponents through `MetaOptimizer.build_opponents()` and `store_night` trains a
`MetaOptimizer` with the shared `CandidatePool`; neither passes `training_gate`,
so a library deck holding a non-admissible card is still an opponent and the gate
artifacts are never read (a closed mechanics gate cannot 500 the endpoint).
"""
import json

import pytest

import digimon_gym.agents.architect_optimizer as architect_optimizer
import digimon_gym.agents.architect_simulator as architect_simulator
import digimon_gym.agents.gauntlet as gauntlet
from server.routers import deck_optimizer

BAD = "BT12-031"  # untested_lowering in the committed artifact
GOOD = "BT12-022"


def _never(path=None):
    raise AssertionError("a hosted deck tool must not read the training gate artifacts")


def test_evaluate_builds_opponents_without_the_training_gate(tmp_path, monkeypatch):
    opp_deck = [GOOD] * 49 + [BAD] + ["BT12-002"] * 5
    library = tmp_path / "deck_library.json"
    library.write_text(
        json.dumps({"archetypes": {"Opp": {"decklists": [{"source": "digilab", "decklist": json.dumps(opp_deck)}]}}}),
        encoding="utf-8",
    )
    (tmp_path / "meta.json").write_text(
        json.dumps({"archetypes": {"Opp": {"global_meta_share": 1.0}}}), encoding="utf-8"
    )
    monkeypatch.setattr(architect_optimizer, "_DECK_LIBRARY_PATH", library)
    monkeypatch.setattr(deck_optimizer, "ARCHITECT_MODELS_DIR", str(tmp_path))
    monkeypatch.setattr(gauntlet, "_load_mechanics_readiness", _never)
    monkeypatch.setattr(gauntlet, "_load_card_admissibility", _never)

    seen = {}

    class FakeSimulator:
        def __init__(self, **kwargs):
            pass

        def evaluate_deck(self, deck, opponents):
            seen["opponents"] = opponents
            return 0.5

    monkeypatch.setattr(architect_simulator, "DeckSimulator", FakeSimulator)
    resp = deck_optimizer.evaluate_deck(
        deck_optimizer.EvaluateRequest(deck=[GOOD] * 50 + ["BT12-002"] * 5, meta_config_name="meta", games=2)
    )
    assert resp.win_rate == 0.5
    assert seen["opponents"] == [(opp_deck, 1.0)], "the inadmissible library deck stays an opponent"


def test_no_server_module_passes_the_training_gate():
    """Grep guard: nothing under code/server opts into the training gate."""
    from pathlib import Path

    server = Path(deck_optimizer.__file__).resolve().parents[1]
    offenders = [
        str(p) for p in server.rglob("*.py")
        if "training_gate" in p.read_text(encoding="utf-8", errors="replace")
        or "architect_admission" in p.read_text(encoding="utf-8", errors="replace")
    ]
    assert offenders == []
