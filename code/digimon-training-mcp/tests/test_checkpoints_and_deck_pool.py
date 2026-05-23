"""Tests for ``run_checkpoints`` and ``run_deck_pool``."""

from __future__ import annotations

from digimon_training_mcp.checkpoints import run_checkpoints
from digimon_training_mcp.deck_pool import run_deck_pool


def test_run_checkpoints_lists_in_step_order(synthetic_repo):
    result = run_checkpoints(synthetic_repo["models_dir"], "active_run")
    assert result["ok"] is True
    assert result["model_run_id"] == "pilot_ppo_20260523_100355"
    steps = [c["step"] for c in result["checkpoints"]]
    assert steps == [100_000, 200_000, 300_000]


def test_run_checkpoints_includes_size_mb(synthetic_repo):
    result = run_checkpoints(synthetic_repo["models_dir"], "active_run")
    for entry in result["checkpoints"]:
        assert isinstance(entry["size_mb"], float)
        assert entry["size_mb"] > 0


def test_run_checkpoints_unknown_run(synthetic_repo):
    result = run_checkpoints(synthetic_repo["models_dir"], "nonexistent")
    assert result["ok"] is False


def test_run_deck_pool_returns_verbatim_content(synthetic_repo):
    result = run_deck_pool(synthetic_repo["models_dir"], "active_run")
    assert result["ok"] is True
    assert result["model_run_id"] == "pilot_ppo_20260523_100355"
    assert len(result["archetypes"]) == 4
    assert result["deck_count"] == 8
    assert len(result["decks"]) == 8
    assert "Medusamon" in result["archetypes"]


def test_run_deck_pool_missing_file_returns_structured_error(synthetic_repo):
    """stale_run has no deck_pool_snapshot.json."""
    result = run_deck_pool(synthetic_repo["models_dir"], "stale_run")
    assert result["ok"] is False
    assert "not found" in result["error"]
