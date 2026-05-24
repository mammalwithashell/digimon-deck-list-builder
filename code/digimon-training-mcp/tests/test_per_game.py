"""Tests for the ``run_per_game_evals`` MCP tool.

Builds on the ``synthetic_repo`` fixture (which already creates two runs
with nested + flat model-side layouts) and seeds ``eval_game_log.jsonl``
files into them.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List

import pytest

from digimon_training_mcp.per_game import run_per_game_evals, has_eval_game_log
from digimon_training_mcp.runs import list_runs


def _write_log(path: Path, rows: List[Dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row) + "\n")


def _row(
    *,
    step: int = 100_000,
    eval_window_idx: int = 0,
    game_idx: int = 0,
    agent_archetype: str = "BlueFlare",
    opponent_archetype: str = "Omnimon",
    digivolves_agent: int = 1,
    dna_digivolves_agent: int = 0,
    digivolves_opponent: int = 1,
    dna_digivolves_opponent: int = 0,
    result: str = "win",
    episode_length: int = 18,
    terminal_score: float = 1.0,
    recording_path: str | None = None,
) -> Dict[str, Any]:
    return {
        "step": step,
        "eval_window_idx": eval_window_idx,
        "game_idx": game_idx,
        "source": "eval",
        "agent_archetype": agent_archetype,
        "opponent_archetype": opponent_archetype,
        "digivolves_agent": digivolves_agent,
        "dna_digivolves_agent": dna_digivolves_agent,
        "digivolves_opponent": digivolves_opponent,
        "dna_digivolves_opponent": dna_digivolves_opponent,
        "result": result,
        "episode_length": episode_length,
        "terminal_score": terminal_score,
        "recording_path": recording_path,
    }


def test_returns_sorted_rows(synthetic_repo):
    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    rows_to_write = [
        _row(step=200_000, eval_window_idx=1, game_idx=1),
        _row(step=100_000, eval_window_idx=0, game_idx=1),
        _row(step=200_000, eval_window_idx=1, game_idx=0),
        _row(step=100_000, eval_window_idx=0, game_idx=0),
    ]
    _write_log(log_path, rows_to_write)

    result = run_per_game_evals(synthetic_repo["models_dir"], "active_run")
    assert result["ok"] is True
    assert result["model_run_id"] == "pilot_ppo_20260523_100355"
    assert result["count"] == 4
    keys = [(r["step"], r["eval_window_idx"], r["game_idx"]) for r in result["rows"]]
    assert keys == sorted(keys)


def test_filter_combines_and_style(synthetic_repo):
    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    rows_to_write = [
        _row(agent_archetype="BlueFlare", result="win", dna_digivolves_agent=2, game_idx=0),
        _row(agent_archetype="BlueFlare", result="loss", dna_digivolves_agent=2, game_idx=1),
        _row(agent_archetype="BlueFlare", result="win", dna_digivolves_agent=0, game_idx=2),
        _row(agent_archetype="Omnimon", result="win", dna_digivolves_agent=2, game_idx=3),
    ]
    _write_log(log_path, rows_to_write)

    result = run_per_game_evals(
        synthetic_repo["models_dir"],
        "active_run",
        {
            "agent_archetype": "BlueFlare",
            "result": "win",
            "dna_digivolves_agent_min": 1,
        },
    )
    assert result["count"] == 1
    assert result["rows"][0]["game_idx"] == 0


def test_digivolves_agent_min_surfaces_whale_games(synthetic_repo):
    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    rows_to_write = [
        _row(digivolves_agent=0, game_idx=0),
        _row(digivolves_agent=1, game_idx=1),
        _row(digivolves_agent=4, game_idx=2),
        _row(digivolves_agent=5, game_idx=3),
    ]
    _write_log(log_path, rows_to_write)

    result = run_per_game_evals(
        synthetic_repo["models_dir"],
        "active_run",
        {"digivolves_agent_min": 3},
    )
    assert result["count"] == 2
    game_idxs = {r["game_idx"] for r in result["rows"]}
    assert game_idxs == {2, 3}


def test_limit_truncates_post_sort(synthetic_repo):
    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    rows_to_write = [_row(step=100_000 + i * 1000, game_idx=i) for i in range(20)]
    _write_log(log_path, rows_to_write)

    result = run_per_game_evals(
        synthetic_repo["models_dir"], "active_run", None, limit=5
    )
    assert result["count"] == 5
    # First 5 by sort order.
    assert [r["game_idx"] for r in result["rows"]] == [0, 1, 2, 3, 4]


def test_step_range_filter(synthetic_repo):
    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    rows_to_write = [
        _row(step=50_000, game_idx=0),
        _row(step=150_000, game_idx=1),
        _row(step=250_000, game_idx=2),
        _row(step=350_000, game_idx=3),
    ]
    _write_log(log_path, rows_to_write)

    result = run_per_game_evals(
        synthetic_repo["models_dir"],
        "active_run",
        {"step_min": 100_000, "step_max": 300_000},
    )
    assert result["count"] == 2
    assert {r["game_idx"] for r in result["rows"]} == {1, 2}


def test_missing_file_returns_empty_rows(synthetic_repo):
    # active_run has a nested model dir but no eval_game_log.jsonl yet.
    result = run_per_game_evals(synthetic_repo["models_dir"], "active_run")
    assert result["ok"] is True
    assert result["count"] == 0
    assert result["rows"] == []


def test_unknown_run_returns_empty_rows(synthetic_repo):
    result = run_per_game_evals(synthetic_repo["models_dir"], "no_such_run")
    assert result["ok"] is True
    assert result["count"] == 0
    assert result["rows"] == []


def test_timestamped_subdir_resolution(synthetic_repo):
    # The fixture's "active_run" has a nested layout (pilot_ppo_<ts>/).
    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    _write_log(log_path, [_row(game_idx=0), _row(game_idx=1)])

    result = run_per_game_evals(synthetic_repo["models_dir"], "active_run")
    assert result["model_run_id"] == "pilot_ppo_20260523_100355"
    assert result["count"] == 2


def test_flat_layout_resolution(synthetic_repo):
    # The fixture's "stale_run" has a flat layout (markers directly).
    log_path = synthetic_repo["models_dir"] / "stale_run" / "eval_game_log.jsonl"
    _write_log(log_path, [_row(game_idx=0)])

    result = run_per_game_evals(synthetic_repo["models_dir"], "stale_run")
    assert result["model_run_id"] == "stale_run"
    assert result["count"] == 1


def test_has_eval_game_log_flag_in_list_runs(synthetic_repo):
    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    _write_log(log_path, [_row(game_idx=0)])

    result = list_runs(synthetic_repo["runs_dir"], synthetic_repo["models_dir"])
    assert result["ok"] is True
    by_name = {r["name"]: r for r in result["runs"]}
    assert by_name["active_run"]["has_eval_game_log"] is True
    assert by_name["stale_run"]["has_eval_game_log"] is False


def test_recording_path_round_trips(synthetic_repo):
    rec_path = synthetic_repo["nested_model_run"] / "recordings" / "test_rec.json"
    rec_path.parent.mkdir(parents=True, exist_ok=True)
    rec_path.write_text("{}", encoding="utf-8")

    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    _write_log(log_path, [_row(recording_path=str(rec_path))])

    result = run_per_game_evals(synthetic_repo["models_dir"], "active_run")
    returned_path = result["rows"][0]["recording_path"]
    assert returned_path == str(rec_path)
    assert Path(returned_path).is_file()


def test_tolerates_partial_last_line(synthetic_repo):
    log_path = synthetic_repo["nested_model_run"] / "eval_game_log.jsonl"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("w", encoding="utf-8") as fh:
        fh.write(json.dumps(_row(game_idx=0)) + "\n")
        fh.write(json.dumps(_row(game_idx=1)) + "\n")
        fh.write('{"partial": tru')  # truncated mid-row

    result = run_per_game_evals(synthetic_repo["models_dir"], "active_run")
    assert result["count"] == 2
    assert {r["game_idx"] for r in result["rows"]} == {0, 1}
