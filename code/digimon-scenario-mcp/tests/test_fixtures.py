"""Fixture file-op + assertion-editing coverage (no server needed)."""

from __future__ import annotations

import json
from pathlib import Path

from digimon_scenario_mcp import fixtures as fx


def _write(dir_: Path, name: str, data: dict) -> None:
    (dir_ / f"{name}.json").write_text(json.dumps(data), encoding="utf-8")


def test_list_load_save_round_trip(tmp_path: Path) -> None:
    _write(tmp_path, "alpha", {"title": "Alpha board", "readiness": "expected_pass"})

    listed = fx.list_fixtures(tmp_path)
    assert listed["ok"] is True
    names = [f["name"] for f in listed["fixtures"]]
    assert "alpha" in names

    loaded = fx.load_fixture(tmp_path, "alpha")
    assert loaded["ok"] is True
    assert loaded["fixture"]["title"] == "Alpha board"

    saved = fx.save_fixture(tmp_path, "beta", {"state": {"memory": 1}})
    assert saved["ok"] is True
    on_disk = json.loads((tmp_path / "beta.json").read_text(encoding="utf-8"))
    # save stamps the id from the name.
    assert on_disk["id"] == "beta"
    assert on_disk["state"]["memory"] == 1


def test_load_missing_fixture_is_clean_error(tmp_path: Path) -> None:
    res = fx.load_fixture(tmp_path, "nope")
    assert res["ok"] is False
    assert "not found" in res["error"]


def test_save_rejects_unsafe_names(tmp_path: Path) -> None:
    for bad in ("../escape", "a/b", ".hidden"):
        res = fx.save_fixture(tmp_path, bad, {})
        assert res["ok"] is False, bad


def test_add_assertion_appends_to_bucket() -> None:
    base = {"assertions": {"engine": [], "ui": []}}
    res = fx.add_assertion(base, {"kind": "memory_equals", "value": 3}, "engine")
    assert res["ok"] is True
    assert res["fixture"]["assertions"]["engine"] == [{"kind": "memory_equals", "value": 3}]
    # Original not mutated (deep copy).
    assert base["assertions"]["engine"] == []


def test_add_assertion_creates_missing_buckets() -> None:
    res = fx.add_assertion({}, {"kind": "stack_top"}, "engine")
    assert res["ok"] is True
    assert res["fixture"]["assertions"]["engine"] == [{"kind": "stack_top"}]


def test_add_assertion_rejects_bad_bucket() -> None:
    res = fx.add_assertion({}, {"kind": "x"}, "nonsense")
    assert res["ok"] is False
