"""Capture-route coverage for the add-scenario-capture-mcp change (task 2.3).

Stages a board via /debug, captures it through POST
/games/{id}/export-scenario (the engine-only export route), then re-stages
the captured fixture via /debug and asserts the two boards' full-information
internal-state matches. This pins that `to_scenario()` is a faithful
round-trip over HTTP.
"""

from __future__ import annotations

from fastapi.testclient import TestClient

from server.api import app

_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45

# Paildramon (AD1-011) over ExVeemon (BT12-022) + Stingmon (BT12-050).
_FIELD = {"stack": ["BT12-022", "BT12-050", "AD1-011"], "is_suspended": False, "turn_played": 0}


def _create_debug(client: TestClient, body: dict) -> str:
    resp = client.post("/debug/games", json=body)
    assert resp.status_code == 200, resp.text
    return resp.json()["game_id"]


def _internal_state(client: TestClient, gid: str) -> dict:
    r = client.get(f"/debug/games/{gid}/internal-state")
    assert r.status_code == 200, r.text
    return r.json()


def test_export_scenario_round_trips_over_http() -> None:
    client = TestClient(app)

    # Stage board A.
    gid_a = _create_debug(
        client,
        {
            "deck1": _DECK,
            "deck2": _DECK,
            "phase": "Main",
            "initial_memory": 2,
            "first_player": 1,
            "turn": 4,
            "zones": {"1": {"field": [_FIELD], "trash": ["ST1-03"]}},
        },
    )

    # Capture A through the new export route (drives the same active_games
    # entry the /games routes use, so /games/{id}/export-scenario resolves
    # the debug game).
    resp = client.post(f"/games/{gid_a}/export-scenario")
    assert resp.status_code == 200, resp.text
    fixture = resp.json()

    # The captured fixture must carry the documented schema shape.
    assert fixture["schema_version"] == 1
    for key in ("decks", "zones", "state", "assertions"):
        assert key in fixture, f"captured fixture missing {key!r}"
    assert fixture["state"]["memory"] == 2
    assert fixture["state"]["turn"] == 4
    assert fixture["state"]["phase"] == "Main"
    # The staged stack survived capture.
    assert fixture["zones"]["1"]["field"][0]["stack"] == ["BT12-022", "BT12-050", "AD1-011"]

    # Re-stage board B from the captured fixture.
    gid_b = _create_debug(
        client,
        {
            "deck1": fixture["decks"]["1"],
            "deck2": fixture["decks"]["2"],
            "phase": fixture["state"]["phase"],
            "initial_memory": fixture["state"]["memory"],
            "first_player": fixture["state"]["first_player"],
            "turn": fixture["state"]["turn"],
            "zones": fixture["zones"],
        },
    )

    state_a = _internal_state(client, gid_a)
    state_b = _internal_state(client, gid_b)

    # Scalar state matches.
    for key in ("memory", "turn_count", "current_phase"):
        assert state_a[key] == state_b[key], f"{key} diverged: {state_a[key]} vs {state_b[key]}"

    # Per-player zones match (full-information).
    for pa, pb in zip(state_a["players"], state_b["players"]):
        assert pa["hand"] == pb["hand"], "hand diverged"
        assert pa["trash"] == pb["trash"], "trash diverged"
        assert pa["battle_area"] == pb["battle_area"], "battle_area diverged"
        assert pa["breeding"] == pb["breeding"], "breeding diverged"
        assert pa["deck_count"] == pb["deck_count"], "deck_count diverged"
        assert pa["security_count"] == pb["security_count"], "security_count diverged"

    for gid in (gid_a, gid_b):
        client.delete(f"/debug/games/{gid}")
