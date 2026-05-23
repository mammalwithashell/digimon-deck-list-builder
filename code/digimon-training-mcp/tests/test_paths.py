"""Tests for path resolution — ancestor-walk + model-run resolution."""

from __future__ import annotations

import os
from pathlib import Path

from digimon_training_mcp.paths import (
    resolve_model_run_dir,
    resolve_models_dir,
    resolve_runs_dir,
)


def test_resolve_runs_dir_with_explicit_override(synthetic_repo):
    runs = resolve_runs_dir(synthetic_repo["runs_dir"])
    assert runs == synthetic_repo["runs_dir"]


def test_resolve_runs_dir_falls_back_to_ancestor_walk(synthetic_repo, monkeypatch):
    """cwd two levels above the synthetic runs dir → walk finds it."""
    deep = synthetic_repo["repo_root"] / "qa" / "archetype-qa"
    monkeypatch.chdir(deep)
    resolved = resolve_runs_dir(None)
    assert resolved == synthetic_repo["runs_dir"]


def test_resolve_runs_dir_returns_none_when_override_missing(tmp_path):
    assert resolve_runs_dir(tmp_path / "missing") is None


def test_resolve_models_dir_with_explicit_override(synthetic_repo):
    models = resolve_models_dir(synthetic_repo["models_dir"])
    assert models == synthetic_repo["models_dir"]


def test_resolve_model_run_dir_nested_layout(synthetic_repo):
    resolved = resolve_model_run_dir(synthetic_repo["models_dir"], "active_run")
    assert resolved is not None
    run_dir, model_run_id = resolved
    assert run_dir == synthetic_repo["nested_model_run"]
    assert model_run_id == "pilot_ppo_20260523_100355"


def test_resolve_model_run_dir_flat_layout(synthetic_repo):
    resolved = resolve_model_run_dir(synthetic_repo["models_dir"], "stale_run")
    assert resolved is not None
    run_dir, model_run_id = resolved
    assert run_dir == synthetic_repo["models_dir"] / "stale_run"
    assert model_run_id == "stale_run"


def test_resolve_model_run_dir_unknown_name(synthetic_repo):
    assert resolve_model_run_dir(synthetic_repo["models_dir"], "nonexistent") is None
