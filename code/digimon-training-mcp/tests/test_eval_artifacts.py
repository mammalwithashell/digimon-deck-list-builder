"""Tests for the eval-artifact readers (champion standings, Elo ladder,
exploitability). Part of add-model-evaluation-harness (Phase 5)."""
import json

from digimon_training_mcp.eval_artifacts import (
    champion_standings, run_elo_ladder, run_exploitability,
)


def test_champion_standings_reads_registry(tmp_path):
    champ_dir = tmp_path / "champions"
    champ_dir.mkdir()
    (champ_dir / "registry.json").write_text(json.dumps({
        "version": 1,
        "champions": [
            {"name": "v22", "weights_path": "/m/v22.zip", "algorithm": "lstm",
             "observation_profile": "standard_lite_deck_v2",
             "tensor_layout_hash": "sha256:abc", "source": "cloud", "created_at": "2026-05-31"},
        ],
    }))
    out = champion_standings(tmp_path)
    assert out["ok"] and out["count"] == 1
    assert out["champions"][0]["name"] == "v22"


def test_champion_standings_missing_registry_is_empty(tmp_path):
    out = champion_standings(tmp_path)
    assert out["ok"] and out["champions"] == []


def test_champion_standings_no_models_dir():
    assert champion_standings(None)["ok"] is False


def test_run_elo_ladder_reads_persisted(tmp_path):
    run = tmp_path / "pilot_ppo_x"
    run.mkdir()
    (run / "elo_ladder.json").write_text(json.dumps({
        "ratings": {"greedy": 1000.0, "step_500k": 1150.0},
        "upsets": [],
    }))
    out = run_elo_ladder(tmp_path, "pilot_ppo_x")
    assert out["ok"] and out["run"] == "pilot_ppo_x"
    assert out["ratings"]["step_500k"] == 1150.0


def test_run_elo_ladder_missing(tmp_path):
    assert run_elo_ladder(tmp_path, "nope")["ok"] is False


def test_run_exploitability_marks_lower_bound(tmp_path):
    run = tmp_path / "exploit_run"
    run.mkdir()
    (run / "exploitability.json").write_text(json.dumps({
        "target": "v22", "exploitability": 0.62, "budget_timesteps": 500000,
    }))
    out = run_exploitability(tmp_path, "exploit_run")
    assert out["ok"] and out["exploitability"] == 0.62
    assert out["is_lower_bound"] is True
