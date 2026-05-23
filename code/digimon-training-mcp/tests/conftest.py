"""Shared fixtures for digimon-training-mcp tests.

The ``synthetic_repo`` fixture builds a self-contained repo-shaped tempdir
with two runs, model artifacts, and a panic-families.json file — designed to
exercise every spec scenario without touching real training output.
"""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Dict, List

import pytest


# ─── Helper builders ─────────────────────────────────────────────────


HEADER_BANNER = "=" * 60


def _write_console_log(
    path: Path,
    *,
    algo: str = "MaskablePPO",
    opponent: str = "greedy",
    eval_lines: List[Dict] = None,
    panic_lines: List[str] = None,
    extra_lines: List[str] = None,
) -> None:
    eval_lines = eval_lines or []
    panic_lines = panic_lines or []
    extra_lines = extra_lines or []
    lines = [
        HEADER_BANNER,
        "Digimon TCG Pilot Agent Training",
        HEADER_BANNER,
        f"  Algorithm:      {algo}",
        f"  Opponent:       {opponent}",
        "  Total steps:    1,000,000",
        "  Learning rate:  0.0003",
        "  Batch size:     64",
        "  Rollout steps:  256",
        "  Eval freq:      every 10,000 steps",
        "  TensorBoard:    runs/test_run",
        "  Tensor profile: standard_lite_v2",
        "  Tensor layout:  size=8320, tensor_v=2, schema=2",
        "  Layout hash:    abc123def456",
        "  Gauntlet:       4 archetypes, 8 decks",
        HEADER_BANNER,
    ]
    lines.extend(extra_lines)
    for eval_row in eval_lines:
        step = eval_row["step"]
        wr = eval_row["win_rate"] * 100
        mr = eval_row["mean_reward"]
        gp = eval_row["games_played"]
        lines.append(f"  [Eval @ {step:,} steps] Win rate: {wr:.1f}% | Mean reward: {mr:.3f} | Games played: {gp:,}")
    for panic_msg in panic_lines:
        lines.append(f"[recorder env=0 game=42 step=128] ENGINE PANIC #1: PanicException: {panic_msg}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _write_evals_jsonl(path: Path, rows: List[Dict]) -> None:
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, separators=(",", ":")) + "\n")


def _write_synthetic_tb_events(tb_subdir: Path, tag_values: Dict[str, List[Dict]]) -> None:
    """Write a tiny TB event file. Uses torch's SummaryWriter to keep deps lean."""
    from torch.utils.tensorboard import SummaryWriter

    tb_subdir.mkdir(parents=True, exist_ok=True)
    writer = SummaryWriter(log_dir=str(tb_subdir), flush_secs=1)
    for tag, points in tag_values.items():
        for point in points:
            writer.add_scalar(tag, point["value"], global_step=point["step"], walltime=point.get("wall_time"))
    writer.flush()
    writer.close()


def _write_recording(path: Path, *, result: str, reason: str, step_count: int = 50) -> None:
    artifact = {
        "schema_version": 1,
        "created_at": "2026-05-23T00:00:00+00:00",
        "run": {"source": path.stem.split("_")[0], "env_index": 0, "game_index": 0},
        "outcome": {
            "result": result,
            "reason": reason,
            "step_count": step_count,
        },
        "recording": {"initial_state": {}, "actions": [], "total_actions": 0},
    }
    path.write_text(json.dumps(artifact), encoding="utf-8")


# ─── Fixture ─────────────────────────────────────────────────────────


@pytest.fixture
def synthetic_repo(tmp_path: Path) -> Dict[str, Path]:
    """Build a synthetic repo with two runs and corresponding model artifacts.

    Layout::

        tmp_path/
        ├── qa/archetype-qa/panic-families.json   (3 known families)
        ├── runs/
        │   ├── active_run/
        │   │   ├── console.log         (header + 3 evals + 4 panic lines)
        │   │   ├── evals.jsonl         (5 rows, latest_step=50000)
        │   │   └── MaskablePPO_1/events.out.tfevents.*
        │   └── stale_run/
        │       └── console.log         (header only, no evals)
        └── models/
            ├── active_run/                       (nested layout)
            │   └── pilot_ppo_20260523_100355/
            │       ├── recordings/   (15 crash + 10 draw/step_limit + 25 win files)
            │       ├── checkpoints/  (step_000100000.zip, step_000200000.zip)
            │       └── deck_pool_snapshot.json
            └── stale_run/                         (flat layout — markers directly)
                └── recordings/   (3 win files)
    """
    repo = tmp_path
    runs_dir = repo / "runs"
    models_dir = repo / "models"
    runs_dir.mkdir()
    models_dir.mkdir()

    # ── Panic families JSON ───────────────────────────────────────
    qa_dir = repo / "qa" / "archetype-qa"
    qa_dir.mkdir(parents=True)
    (qa_dir / "panic-families.json").write_text(json.dumps({
        "$schema_version": 1,
        "families": [
            {
                "family_id": "G-DSL-OUTER-TAIL-NESTED-PARK",
                "description": "Nested outer-tail park",
                "pattern": "dsl_outer_tail overwrite",
                "status": "resolved",
            },
            {
                "family_id": "G-OPTION-PLAY-REENTRANT",
                "description": "Reentrant Option play",
                "pattern": "reentrant Option play",
                "status": "resolved",
            },
            {
                "family_id": "G-DELETION-RESUME-NESTED",
                "description": "Nested deferred deletion",
                "pattern": "nested deferred deletion",
                "status": "open",
            },
        ],
    }), encoding="utf-8")

    # ── active_run ────────────────────────────────────────────────
    active_run = runs_dir / "active_run"
    active_run.mkdir()
    eval_rows = [
        {"step": 10_000, "wall_time": 1716000000.0, "win_rate": 0.45,
         "mean_reward": 0.12, "draw_rate": 0.05, "mean_terminal_score": 0.4,
         "mean_dense_reward": 0.0, "mean_eval_episode_length": 78.0, "games_played": 100},
        {"step": 20_000, "wall_time": 1716000100.0, "win_rate": 0.52,
         "mean_reward": 0.20, "draw_rate": 0.04, "mean_terminal_score": 0.5,
         "mean_dense_reward": 0.0, "mean_eval_episode_length": 76.0, "games_played": 200},
        {"step": 30_000, "wall_time": 1716000200.0, "win_rate": 0.58,
         "mean_reward": 0.28, "draw_rate": 0.03, "mean_terminal_score": 0.58,
         "mean_dense_reward": 0.0, "mean_eval_episode_length": 74.0, "games_played": 300},
        {"step": 40_000, "wall_time": 1716000300.0, "win_rate": 0.63,
         "mean_reward": 0.33, "draw_rate": 0.02, "mean_terminal_score": 0.63,
         "mean_dense_reward": 0.0, "mean_eval_episode_length": 72.0, "games_played": 400},
        {"step": 50_000, "wall_time": 1716000400.0, "win_rate": 0.68,
         "mean_reward": 0.39, "draw_rate": 0.02, "mean_terminal_score": 0.68,
         "mean_dense_reward": 0.0, "mean_eval_episode_length": 70.0, "games_played": 500},
    ]
    _write_evals_jsonl(active_run / "evals.jsonl", eval_rows)
    _write_console_log(
        active_run / "console.log",
        algo="MaskablePPO",
        opponent="greedy",
        eval_lines=eval_rows[:3],
        panic_lines=[
            "dsl_outer_tail overwrite: card=BT24-016 player=1",
            "dsl_outer_tail overwrite: card=BT24-016 player=1 parking_step=AsSelectingPlayer",
            "reentrant Option play while another is mid-resolution: card=P-103",
            "some weird unmatched panic that isn't in our family table",
        ],
        extra_lines=["  Some extra stdout line", "  Another line"],
    )
    _write_synthetic_tb_events(
        active_run / "MaskablePPO_1",
        {
            "pilot/win_rate": [{"step": r["step"], "value": r["win_rate"]} for r in eval_rows],
            "rollout/ep_rew_mean": [{"step": r["step"], "value": r["mean_reward"]} for r in eval_rows],
            "train/loss": [{"step": r["step"], "value": 1.0 - r["win_rate"]} for r in eval_rows],
        },
    )

    # ── stale_run (header only, ancient mtime) ───────────────────
    stale_run = runs_dir / "stale_run"
    stale_run.mkdir()
    _write_console_log(stale_run / "console.log", algo="MaskablePPO", opponent="random")
    ancient = time.time() - 7200  # 2 hours ago — well outside the active window
    import os
    os.utime(stale_run / "console.log", (ancient, ancient))
    os.utime(stale_run, (ancient, ancient))

    # ── models/active_run/ — nested layout ───────────────────────
    nested_model_run = models_dir / "active_run" / "pilot_ppo_20260523_100355"
    rec_dir = nested_model_run / "recordings"
    rec_dir.mkdir(parents=True)
    for i in range(15):
        _write_recording(rec_dir / f"train_env_000_game_{i:06d}_draw_crash.json", result="draw", reason="crash")
    for i in range(15, 25):
        _write_recording(rec_dir / f"train_env_001_game_{i:06d}_draw_step_limit.json", result="draw", reason="step_limit")
    for i in range(25, 50):
        _write_recording(rec_dir / f"train_env_002_game_{i:06d}_win_decked_out.json", result="win", reason="decked_out")

    ckpt_dir = nested_model_run / "checkpoints"
    ckpt_dir.mkdir()
    for step in (100_000, 200_000, 300_000):
        (ckpt_dir / f"step_{step:09d}.zip").write_bytes(b"\x50\x4b\x03\x04" + b"\x00" * 1024)

    (nested_model_run / "deck_pool_snapshot.json").write_text(json.dumps({
        "archetypes": ["Medusamon", "BlackWargreymon", "Tyrant", "Imperialdramon"],
        "decks": [{"name": f"deck_{i}", "cards": []} for i in range(8)],
    }), encoding="utf-8")

    # ── models/stale_run/ — flat layout (markers directly) ────────
    flat_model_run = models_dir / "stale_run"
    flat_rec_dir = flat_model_run / "recordings"
    flat_rec_dir.mkdir(parents=True)
    for i in range(3):
        _write_recording(flat_rec_dir / f"train_env_000_game_{i:06d}_win_defeat.json", result="win", reason="defeat")

    return {
        "repo_root": repo,
        "runs_dir": runs_dir,
        "models_dir": models_dir,
        "active_run_dir": active_run,
        "stale_run_dir": stale_run,
        "nested_model_run": nested_model_run,
    }
