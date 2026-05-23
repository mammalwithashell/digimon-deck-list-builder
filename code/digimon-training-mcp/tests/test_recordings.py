"""Tests for ``run_recordings`` — spec §run_recordings scenarios."""

from __future__ import annotations

from digimon_training_mcp.recordings import FILENAME_RE, run_recordings


def test_filename_regex_matches_real_format():
    """Spec scenario: literal filename from engine-gaps.md §G-DELETION-RESUME-NESTED."""
    m = FILENAME_RE.match("train_env_000_game_000034_draw_crash.json")
    assert m is not None
    assert m.group("source") == "train"
    assert m.group("env") == "000"
    assert m.group("game") == "000034"
    assert m.group("result") == "draw"
    assert m.group("reason") == "crash"


def test_filename_regex_handles_multi_word_reasons():
    """Reasons like 'step_limit' / 'decked_out' contain underscores."""
    m = FILENAME_RE.match("train_env_001_game_000099_draw_step_limit.json")
    assert m is not None
    assert m.group("result") == "draw"
    assert m.group("reason") == "step_limit"


def test_run_recordings_all_returns_50_entries(synthetic_repo):
    result = run_recordings(synthetic_repo["models_dir"], "active_run", "all")
    assert result["ok"] is True
    assert result["model_run_id"] == "pilot_ppo_20260523_100355"
    assert result["count"] == 50


def test_run_recordings_crash_filter(synthetic_repo):
    result = run_recordings(synthetic_repo["models_dir"], "active_run", "crash")
    assert result["ok"] is True
    assert result["count"] == 15
    assert all(r["reason"] == "crash" for r in result["recordings"])


def test_run_recordings_draw_filter_excludes_crashes(synthetic_repo):
    result = run_recordings(synthetic_repo["models_dir"], "active_run", "draw")
    assert result["ok"] is True
    assert result["count"] == 10  # step_limit only — crashes are filtered out
    for r in result["recordings"]:
        assert r["result"] == "draw"
        assert r["reason"] != "crash"


def test_run_recordings_limit_truncates_after_filter(synthetic_repo):
    result = run_recordings(synthetic_repo["models_dir"], "active_run", "crash", limit=5)
    assert result["ok"] is True
    assert result["count"] == 5


def test_run_recordings_returns_absolute_paths(synthetic_repo):
    """Spec scenario: paths must be consumable by engine MCP's load_recording."""
    import os.path

    result = run_recordings(synthetic_repo["models_dir"], "active_run", "all", limit=1)
    assert result["count"] == 1
    path = result["recordings"][0]["path"]
    assert os.path.isabs(path)
    assert os.path.isfile(path)


def test_run_recordings_descends_into_timestamped_subdir(synthetic_repo):
    """Spec scenario: nested layout — markers under <models-dir>/<name>/<timestamped>/."""
    result = run_recordings(synthetic_repo["models_dir"], "active_run", "all")
    assert result["model_run_id"] == "pilot_ppo_20260523_100355"


def test_run_recordings_flat_layout(synthetic_repo):
    """Spec scenario: flat layout — markers directly under <models-dir>/<name>/."""
    result = run_recordings(synthetic_repo["models_dir"], "stale_run", "all")
    assert result["ok"] is True
    assert result["model_run_id"] == "stale_run"  # name == model_run_id when flat
    assert result["count"] == 3


def test_run_recordings_unknown_run_returns_structured_error(synthetic_repo):
    result = run_recordings(synthetic_repo["models_dir"], "nonexistent", "all")
    assert result["ok"] is False
    assert "no model artifacts" in result["error"]
