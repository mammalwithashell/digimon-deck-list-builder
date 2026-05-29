"""Regression coverage for the /games scenario-staging knobs folded into
the add-ui-scenario-test-substrate change: `seed`, `action_script`, and
the mask-validation guard on scripted actions.

These are the lightweight precursor to the full /debug staging surface
(design D8). The key invariant under test: a scripted action that is NOT
legal at its point in the sequence must fail loudly (400), because the
engine silently no-ops illegal actions — an unchecked script would
otherwise produce a divergent board with no error.
"""

from __future__ import annotations

from fastapi.testclient import TestClient

from server.api import app

# Minimal valid deck: 5 Digi-Eggs + 45 main, same shape the Rust binding
# tests use. The /games create path hands this straight to the engine.
_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45


def _create(client: TestClient, **overrides) -> dict:
    body = {
        "deck1": _DECK,
        "deck2": _DECK,
        "player1_type": "human",
        "player2_type": "agent",
        "player2_policy": "greedy",
        "seed": 4242,
    }
    body.update(overrides)
    resp = client.post("/games", json=body)
    return resp


def _first_legal_action(mask: list[int]) -> int:
    for i, bit in enumerate(mask):
        if bit == 1:
            return i
    raise AssertionError("no legal action in opening mask")


def test_seed_makes_opening_state_reproducible() -> None:
    client = TestClient(app)
    a = _create(client).json()
    b = _create(client).json()
    assert a["state"]["turnCount"] == b["state"]["turnCount"]
    assert a["state"]["currentPhase"] == b["state"]["currentPhase"]


def test_action_script_advances_and_is_reproducible() -> None:
    client = TestClient(app)
    baseline = _create(client).json()
    legal = _first_legal_action(baseline["action_mask"])

    a = _create(client, action_script=[legal]).json()
    b = _create(client, action_script=[legal]).json()

    # Same seed + same script → identical landing state.
    assert a["state"]["turnCount"] == b["state"]["turnCount"]
    assert a["state"]["currentPhase"] == b["state"]["currentPhase"]

    # The scripted action actually moved the game off the opening state
    # (turn or phase advanced).
    advanced = (
        a["state"]["turnCount"] != baseline["state"]["turnCount"]
        or a["state"]["currentPhase"] != baseline["state"]["currentPhase"]
    )
    assert advanced, "action_script did not change the game state"


def test_illegal_action_script_is_rejected() -> None:
    client = TestClient(app)
    # 9999 is out of the action space; the engine would no-op it, so the
    # mask guard must reject it with 400 rather than silently diverging.
    resp = _create(client, action_script=[9999])
    assert resp.status_code == 400
    assert "not legal" in resp.json()["detail"]
