"""Tests for the ``run_anchored_evals`` reader + ``run_summary`` surfacing
(`harden-training-pipeline` — in-training anchored panel sidecar)."""

from __future__ import annotations

import json
from pathlib import Path

from digimon_training_mcp.anchored import (
    latest_anchored_row,
    read_anchored_rows,
    run_anchored_evals,
)
from digimon_training_mcp.panic_families import load_panic_families
from digimon_training_mcp.summary import run_summary


def _panel_row(step: int, greedy_wr: float) -> dict:
    return {
        "step": step,
        "wall_time": 1716000000.0 + step,
        "panel_idx": step // 25_000 - 1,
        "games_per_anchor": 24,
        "panel_seconds": 95.2,
        "panel_mean_win_rate": greedy_wr,
        "anchors": {
            "greedy": {
                "wins": int(24 * greedy_wr), "losses": 24 - int(24 * greedy_wr),
                "draws": 0, "win_rate": greedy_wr, "failed": False,
            },
        },
        "excluded_champions": [],
    }


def _write_panels(run_dir: Path, rows) -> None:
    with (run_dir / "anchored_evals.jsonl").open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, separators=(",", ":")) + "\n")


def test_run_anchored_evals_most_recent_first_with_limit(tmp_path):
    run_dir = tmp_path / "run"
    run_dir.mkdir()
    _write_panels(run_dir, [_panel_row(25_000, 0.5), _panel_row(50_000, 0.6),
                            _panel_row(75_000, 0.7)])

    out = run_anchored_evals(run_dir, "run", limit=2)
    assert out["ok"] is True
    assert [p["step"] for p in out["panels"]] == [75_000, 50_000]
    assert out["count"] == 2


def test_run_anchored_evals_missing_sidecar_is_ok_with_note(tmp_path):
    run_dir = tmp_path / "run"
    run_dir.mkdir()
    out = run_anchored_evals(run_dir, "run")
    assert out["ok"] is True
    assert out["panels"] == []
    assert "anchored_eval_freq=0" in out["note"]


def test_run_anchored_evals_unresolved_run_dir_errors():
    out = run_anchored_evals(None, "ghost")
    assert out["ok"] is False
    assert "ghost" in out["error"]


def test_read_rows_skips_corrupt_lines(tmp_path):
    run_dir = tmp_path / "run"
    run_dir.mkdir()
    with (run_dir / "anchored_evals.jsonl").open("w", encoding="utf-8") as fh:
        fh.write(json.dumps(_panel_row(25_000, 0.5)) + "\n")
        fh.write("{corrupt json\n")
        fh.write(json.dumps(_panel_row(50_000, 0.6)) + "\n")
    rows = read_anchored_rows(run_dir)
    assert [r["step"] for r in rows] == [25_000, 50_000]


def test_latest_anchored_row(tmp_path):
    run_dir = tmp_path / "run"
    run_dir.mkdir()
    assert latest_anchored_row(run_dir) is None
    _write_panels(run_dir, [_panel_row(25_000, 0.5), _panel_row(50_000, 0.6)])
    assert latest_anchored_row(run_dir)["step"] == 50_000


def test_run_summary_surfaces_latest_anchored(synthetic_repo):
    active_run = synthetic_repo["active_run_dir"]
    _write_panels(active_run, [_panel_row(25_000, 0.55), _panel_row(50_000, 0.61)])
    families = load_panic_families(synthetic_repo["repo_root"])

    out = run_summary(active_run, tail_evals=3, families=families)
    assert out["ok"] is True
    assert out["latest_anchored"]["step"] == 50_000
    assert out["latest_anchored"]["anchors"]["greedy"]["win_rate"] == 0.61


def test_run_summary_latest_anchored_none_without_sidecar(synthetic_repo):
    active_run = synthetic_repo["active_run_dir"]
    families = load_panic_families(synthetic_repo["repo_root"])
    out = run_summary(active_run, tail_evals=3, families=families)
    assert out["latest_anchored"] is None
