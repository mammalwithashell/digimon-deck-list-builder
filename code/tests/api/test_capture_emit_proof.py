"""End-to-end proof for add-scenario-capture-mcp (task 5.3).

Stage a known board → capture it via the export route → add engine
assertions → emit a fixture + Playwright spec via the scenario MCP's
generator → re-stage the captured fixture → evaluate its engine assertions.
Proves the emitted fixture re-stages identically and its assertions hold
(the generated .spec.ts drives the UI with these same assertions; running
Playwright/browsers is out of scope for this unit-level proof).
"""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient

from server.api import app

spec_gen = pytest.importorskip("digimon_scenario_mcp.spec_gen")
fx = pytest.importorskip("digimon_scenario_mcp.fixtures")

_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45
_FIELD = {"stack": ["BT12-022", "BT12-050", "AD1-011"], "is_suspended": False, "turn_played": 0}


def _create(client: TestClient, body: dict) -> str:
    r = client.post("/debug/games", json=body)
    assert r.status_code == 200, r.text
    return r.json()["game_id"]


def test_capture_emit_and_restage_proof(tmp_path) -> None:
    client = TestClient(app)

    # Stage the Q16 Paildramon board.
    gid = _create(
        client,
        {"deck1": _DECK, "deck2": _DECK, "phase": "Main", "initial_memory": 0,
         "first_player": 1, "turn": 4, "zones": {"1": {"field": [_FIELD]}}},
    )

    # Capture it through the export route.
    fixture = client.post(f"/games/{gid}/export-scenario").json()

    # Author assertions onto the captured fixture.
    fixture = fx.add_assertion(
        fixture, {"kind": "stack_top", "player": 1, "field_index": 0, "card_id": "AD1-011"}
    )["fixture"]
    fixture = fx.add_assertion(
        fixture, {"kind": "effective_dp", "player": 1, "field_index": 0, "value": 8000}
    )["fixture"]

    # Emit fixture + Playwright spec.
    scenarios = tmp_path / "qa" / "scenarios"
    e2e = tmp_path / "code" / "frontend" / "e2e"
    scenarios.mkdir(parents=True)
    e2e.mkdir(parents=True)
    res = spec_gen.generate_spec(fixture, "q16-recaptured", scenarios, e2e)
    assert res["ok"], res
    spec_path = e2e / "q16-recaptured.scenario.spec.ts"
    assert spec_path.is_file()
    assert "loadScenario('q16-recaptured')" in spec_path.read_text(encoding="utf-8")

    # Re-stage the captured fixture and evaluate its engine assertions.
    gid2 = _create(
        client,
        {"deck1": fixture["decks"]["1"], "deck2": fixture["decks"]["2"],
         "phase": fixture["state"]["phase"], "initial_memory": fixture["state"]["memory"],
         "first_player": fixture["state"]["first_player"], "turn": fixture["state"]["turn"],
         "zones": fixture["zones"]},
    )
    ev = client.post(
        f"/debug/games/{gid2}/evaluate",
        json={"assertions": fixture["assertions"]["engine"]},
    )
    assert ev.status_code == 200, ev.text
    assert ev.json()["all_passed"] is True, ev.json()

    for g in (gid, gid2):
        client.delete(f"/debug/games/{g}")
