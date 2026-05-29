"""Engine-assertion evaluator coverage for the Rust-backed /debug router
(add-ui-scenario-test-substrate, task 3.4).

Stages a known board via /debug/games and exercises one assertion of each
kind through /debug/games/{id}/evaluate, asserting both the pass and fail
paths. This pins the evaluator that is the single source of truth for
engine-correctness verdicts across the Rust and Playwright layers.
"""

from __future__ import annotations

from fastapi.testclient import TestClient

from server.api import app

# Starter decks (5 eggs + 45 main); the create path hands these to the engine.
_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45

# Paildramon (AD1-011) over ExVeemon (BT12-022) + Stingmon (BT12-050),
# bottom-to-top. Base/effective DP of Paildramon is 8000.
_FIELD = {"stack": ["BT12-022", "BT12-050", "AD1-011"], "is_suspended": False, "turn_played": 0}


def _stage(client: TestClient) -> str:
    resp = client.post(
        "/debug/games",
        json={
            "deck1": _DECK,
            "deck2": _DECK,
            "phase": "Main",
            "initial_memory": 0,
            "first_player": 1,
            "zones": {"1": {"field": [_FIELD]}},
        },
    )
    assert resp.status_code == 200, resp.text
    return resp.json()["game_id"]


def _evaluate(client: TestClient, gid: str, assertions: list[dict]) -> dict:
    r = client.post(f"/debug/games/{gid}/evaluate", json={"assertions": assertions})
    assert r.status_code == 200, r.text
    return r.json()


def test_evaluator_pass_and_fail_paths() -> None:
    client = TestClient(app)
    gid = _stage(client)
    try:
        result = _evaluate(
            client,
            gid,
            [
                # pass cases
                {"kind": "memory_equals", "value": 0},
                {"kind": "stack_top", "player": 1, "field_index": 0, "card_id": "AD1-011"},
                {"kind": "effective_dp", "player": 1, "field_index": 0, "value": 8000},
                {"kind": "zone_count", "player": 1, "zone": "trash", "value": 0},
                {"kind": "action_legal", "action_id": 9999, "expected": False},
                # fail cases
                {"kind": "memory_equals", "value": 99},
                {"kind": "stack_top", "player": 1, "field_index": 0, "card_id": "WRONG-000"},
            ],
        )
        verdicts = [r["passed"] for r in result["results"]]
        assert verdicts == [True, True, True, True, True, False, False], result
        assert result["all_passed"] is False
    finally:
        client.delete(f"/debug/games/{gid}")


def test_evaluator_all_pass_sets_flag() -> None:
    client = TestClient(app)
    gid = _stage(client)
    try:
        result = _evaluate(
            client,
            gid,
            [
                {"kind": "memory_equals", "value": 0},
                {"kind": "stack_top", "player": 1, "field_index": 0, "card_id": "AD1-011"},
            ],
        )
        assert result["all_passed"] is True
        assert all(r["passed"] for r in result["results"])
    finally:
        client.delete(f"/debug/games/{gid}")
