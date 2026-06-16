from __future__ import annotations

from fastapi.testclient import TestClient

from server.api import app

_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45
_MAX_U64 = "18446744073709551615"


def _create_body(**overrides) -> dict:
    body = {
        "deck1": _DECK,
        "deck2": _DECK,
        "player1_type": "human",
        "player2_type": "agent",
        "player2_policy": "greedy",
        "seed": "12345",
    }
    body.update(overrides)
    return body


def test_create_game_accepts_and_returns_lossless_seed_string() -> None:
    client = TestClient(app)

    resp = client.post("/games", json=_create_body(seed=_MAX_U64))

    assert resp.status_code == 200, resp.text
    payload = resp.json()
    assert payload["seed"] == _MAX_U64


def test_state_and_action_payloads_include_effective_seed() -> None:
    client = TestClient(app)
    created = client.post("/games", json=_create_body(seed="4242")).json()
    game_id = created["game_id"]

    state = client.get(f"/games/{game_id}/state")
    assert state.status_code == 200
    assert state.json()["seed"] == "4242"

    legal = next(i for i, bit in enumerate(created["action_mask"]) if bit == 1)
    action = client.post(f"/games/{game_id}/actions", json={"action": legal})
    assert action.status_code == 200, action.text
    assert action.json()["seed"] == "4242"


def test_invalid_seed_values_are_rejected() -> None:
    client = TestClient(app)

    for seed in ["-1", "1.5", "not-a-seed", "18446744073709551616"]:
        resp = client.post("/games", json=_create_body(seed=seed))
        assert resp.status_code == 422
        assert "base-10" in resp.text or "u64" in resp.text
