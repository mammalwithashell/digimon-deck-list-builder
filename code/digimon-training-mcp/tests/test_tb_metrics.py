"""Tests for ``run_metric`` and ``run_tags``."""

from __future__ import annotations

from digimon_training_mcp.tb_metrics import run_metric, run_tags


def test_run_tags_lists_all_scalar_tags(synthetic_repo):
    result = run_tags(synthetic_repo["active_run_dir"])
    assert result["ok"] is True
    tags = result["tags"]
    assert "pilot/win_rate" in tags
    assert "rollout/ep_rew_mean" in tags
    assert "train/loss" in tags


def test_run_metric_single_tag_returns_timeseries(synthetic_repo):
    result = run_metric(synthetic_repo["active_run_dir"], "pilot/win_rate", since_step=None)
    assert result["ok"] is True
    assert result["tag"] == "pilot/win_rate"
    values = result["values"]
    assert len(values) == 5
    # Sorted by step ascending
    steps = [v["step"] for v in values]
    assert steps == sorted(steps)
    # Values match the eval rows we wrote
    assert values[0]["step"] == 10_000
    assert values[-1]["step"] == 50_000


def test_run_metric_since_step_filters(synthetic_repo):
    result = run_metric(synthetic_repo["active_run_dir"], "pilot/win_rate", since_step=30_000)
    assert result["ok"] is True
    values = result["values"]
    assert len(values) == 3  # 30k, 40k, 50k
    assert all(v["step"] >= 30_000 for v in values)


def test_run_metric_multi_tag_returns_dict(synthetic_repo):
    result = run_metric(
        synthetic_repo["active_run_dir"],
        ["pilot/win_rate", "train/loss"],
        since_step=None,
    )
    assert result["ok"] is True
    assert set(result["series"].keys()) == {"pilot/win_rate", "train/loss"}
    assert len(result["series"]["pilot/win_rate"]) == 5
    assert len(result["series"]["train/loss"]) == 5


def test_run_metric_unknown_tag_returns_empty_list(synthetic_repo):
    result = run_metric(synthetic_repo["active_run_dir"], "this/tag/does/not/exist", since_step=None)
    assert result["ok"] is True
    assert result["values"] == []


def test_run_metric_errors_on_missing_run_dir(tmp_path):
    result = run_metric(tmp_path / "does_not_exist", "pilot/win_rate", since_step=None)
    assert result["ok"] is False
    assert "not found" in result["error"]
