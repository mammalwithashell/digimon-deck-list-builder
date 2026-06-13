"""Pure request-shaping, discovery, and spec-generation coverage."""

from __future__ import annotations

import json
from pathlib import Path

from digimon_scenario_mcp import spec_gen
from digimon_scenario_mcp.transport import (
    build_debug_create_body,
    resolve_desktop_base_url,
)


def test_build_debug_create_body_maps_fixture() -> None:
    fixture = {
        "decks": {"1": ["A"], "2": ["B"]},
        "seed": 9,
        "state": {"memory": 3, "phase": "Main", "turn": 5, "first_player": 2},
        "zones": {"1": {"hand": ["A"]}},
    }
    body = build_debug_create_body(fixture)
    assert body["deck1"] == ["A"]
    assert body["deck2"] == ["B"]
    assert body["seed"] == 9
    assert body["initial_memory"] == 3
    assert body["first_player"] == 2
    assert body["turn"] == 5
    assert body["phase"] == "Main"
    assert body["zones"] == {"1": {"hand": ["A"]}}


def test_build_debug_create_body_defaults() -> None:
    body = build_debug_create_body({})
    assert body["deck1"] == []
    assert body["initial_memory"] == 0
    assert body["first_player"] == 1
    assert body["phase"] == "Main"


def test_resolve_desktop_base_url_precedence(monkeypatch, tmp_path: Path) -> None:
    # explicit beats everything
    assert resolve_desktop_base_url("http://h:9/") == "http://h:9"

    # env beats discovery + default
    monkeypatch.setenv("DIGIMON_DEBUG_BRIDGE_URL", "http://env:1234")
    assert resolve_desktop_base_url() == "http://env:1234"
    monkeypatch.delenv("DIGIMON_DEBUG_BRIDGE_URL", raising=False)

    # discovery file is read when present
    disc = tmp_path / "debug_bridge.json"
    disc.write_text(json.dumps({"port": 5180, "base_url": "http://127.0.0.1:5180"}), encoding="utf-8")
    monkeypatch.setattr(
        "digimon_scenario_mcp.transport.desktop_discovery_path", lambda: disc
    )
    assert resolve_desktop_base_url() == "http://127.0.0.1:5180"


def test_resolve_desktop_base_url_default(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.delenv("DIGIMON_DEBUG_BRIDGE_URL", raising=False)
    monkeypatch.setattr(
        "digimon_scenario_mcp.transport.desktop_discovery_path",
        lambda: tmp_path / "absent.json",
    )
    assert resolve_desktop_base_url() == "http://127.0.0.1:5174"


def test_render_spec_ts_is_self_referential() -> None:
    ts = spec_gen.render_spec_ts("my-scenario", {"title": "My board"})
    assert "loadScenario('my-scenario')" in ts
    assert "My board" in ts
    assert "@generated" in ts
    assert "qa/scenarios/my-scenario.json" in ts


def test_render_spec_ts_is_idempotent() -> None:
    fixture = {"title": "T"}
    assert spec_gen.render_spec_ts("s", fixture) == spec_gen.render_spec_ts("s", fixture)


def test_generate_spec_writes_fixture_and_spec(tmp_path: Path) -> None:
    scenarios = tmp_path / "qa" / "scenarios"
    e2e = tmp_path / "code" / "frontend" / "e2e"
    scenarios.mkdir(parents=True)
    e2e.mkdir(parents=True)

    fixture = {
        "title": "Round-trip board",
        "decks": {"1": [], "2": []},
        "assertions": {"engine": [{"kind": "memory_equals", "value": 0}], "ui": []},
    }
    res = spec_gen.generate_spec(fixture, "captured-1", scenarios, e2e)
    assert res["ok"] is True, res
    assert (scenarios / "captured-1.json").is_file()
    spec_file = e2e / "captured-1.scenario.spec.ts"
    assert spec_file.is_file()
    assert "loadScenario('captured-1')" in spec_file.read_text(encoding="utf-8")
