"""Tests for the ``list_runs`` tool — spec §list_runs scenarios."""

from __future__ import annotations

from digimon_training_mcp.runs import list_runs


def test_list_runs_returns_both_runs(synthetic_repo):
    result = list_runs(synthetic_repo["runs_dir"])
    assert result["ok"] is True
    names = [r["name"] for r in result["runs"]]
    assert "active_run" in names
    assert "stale_run" in names


def test_list_runs_sorts_by_last_modified_desc(synthetic_repo):
    result = list_runs(synthetic_repo["runs_dir"])
    runs = result["runs"]
    assert len(runs) >= 2
    # active_run was just written; stale_run is 2 hours old.
    assert runs[0]["name"] == "active_run"
    assert runs[0]["last_modified_epoch"] > runs[1]["last_modified_epoch"]


def test_active_flag_reflects_freshness(synthetic_repo):
    result = list_runs(synthetic_repo["runs_dir"])
    by_name = {r["name"]: r for r in result["runs"]}
    assert by_name["active_run"]["active"] is True
    assert by_name["stale_run"]["active"] is False


def test_latest_step_and_win_rate_from_sidecar(synthetic_repo):
    result = list_runs(synthetic_repo["runs_dir"])
    by_name = {r["name"]: r for r in result["runs"]}
    assert by_name["active_run"]["latest_step"] == 50_000
    assert by_name["active_run"]["latest_win_rate"] == 0.68
    # stale_run has no sidecar → both null
    assert by_name["stale_run"]["latest_step"] is None
    assert by_name["stale_run"]["latest_win_rate"] is None


def test_list_runs_errors_on_missing_runs_dir():
    result = list_runs(None)
    assert result["ok"] is False
    assert "runs_dir" in result["error"]
