"""Cross-target contract test (spec scenario: same fixture stages on either
target → equal board).

This needs BOTH a live hosted API (browser target) AND a running
debug-bridge-enabled desktop app (desktop target), so it is opt-in: set
`SCENARIO_MCP_LIVE=1` and have both surfaces up. Otherwise it skips — there
is no way to drive the real Tauri process headlessly in CI.
"""

from __future__ import annotations

import os

import pytest

from digimon_scenario_mcp.transport import BrowserClient, DesktopClient

pytestmark = pytest.mark.skipif(
    os.environ.get("SCENARIO_MCP_LIVE") != "1",
    reason="set SCENARIO_MCP_LIVE=1 with a live API + desktop bridge to run",
)

_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45
_FIXTURE = {
    "schema_version": 1,
    "decks": {"1": _DECK, "2": _DECK},
    "seed": 12345,
    "state": {"memory": 1, "phase": "Main", "turn": 4, "first_player": 1},
    "zones": {
        "1": {"field": [{"stack": ["BT12-022", "BT12-050", "AD1-011"], "is_suspended": False, "turn_played": 0}]}
    },
    "assertions": {"engine": [], "ui": []},
}


def _p1_field_stacks_browser(internal_state: dict) -> list:
    for p in internal_state["players"]:
        if p["player_id"] == 1:
            return [perm["stack"] for perm in p["battle_area"]]
    return []


def _p1_field_stacks_desktop(scenario: dict) -> list:
    return [fs["stack"] for fs in scenario["zones"]["1"]["field"]]


def test_same_fixture_stages_equal_on_both_targets() -> None:
    browser = BrowserClient(os.environ.get("BROWSER_URL", "http://127.0.0.1:8000"))
    desktop = DesktopClient()
    if not desktop.health():
        pytest.skip("desktop bridge not reachable")

    try:
        b = browser.stage(_FIXTURE)
        b_state = browser.read_state(b["game_id"])
        desktop.stage(_FIXTURE)
        d_state = desktop.read_state()
    finally:
        browser.close()
        desktop.close()

    assert _p1_field_stacks_browser(b_state) == _p1_field_stacks_desktop(d_state)
