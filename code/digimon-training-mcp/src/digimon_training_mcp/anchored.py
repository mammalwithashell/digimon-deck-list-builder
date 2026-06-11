"""Readers for ``anchored_evals.jsonl`` — the in-training anchored panel
sidecar written by ``AnchoredEvalCallback`` (`harden-training-pipeline` D2).

One JSON row per panel: ``{step, wall_time, panel_idx, games_per_anchor,
panel_seconds, panel_mean_win_rate, anchors: {name: {wins, losses, draws,
win_rate, failed}}, excluded_champions}``. Lives next to ``evals.jsonl`` in
the run directory under ``runs/``.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional

ANCHORED_SIDECAR = "anchored_evals.jsonl"


def read_anchored_rows(run_dir: Path) -> Optional[List[Dict[str, Any]]]:
    """All rows from ``anchored_evals.jsonl``, in file order.

    Returns ``None`` when the sidecar doesn't exist (run predates the
    feature or the panel was disabled), ``[]`` for an empty file.
    """
    path = run_dir / ANCHORED_SIDECAR
    if not path.is_file():
        return None
    rows: List[Dict[str, Any]] = []
    try:
        with path.open("r", encoding="utf-8") as fh:
            for raw in fh:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    rows.append(json.loads(raw))
                except json.JSONDecodeError:
                    continue
    except OSError:
        return []
    return rows


def latest_anchored_row(run_dir: Path) -> Optional[Dict[str, Any]]:
    """The most recent panel row, or ``None`` when there is none."""
    rows = read_anchored_rows(run_dir)
    if not rows:
        return None
    return rows[-1]


def run_anchored_evals(
    run_dir: Optional[Path],
    name: str,
    limit: Optional[int] = None,
) -> Dict[str, Any]:
    """``run_anchored_evals`` tool body: most-recent-first rows with an
    optional ``limit``."""
    if run_dir is None:
        return {"ok": False, "error": f"run '{name}' not found under runs_dir"}
    rows = read_anchored_rows(run_dir)
    if rows is None:
        return {
            "ok": True,
            "run": name,
            "panels": [],
            "note": (
                f"no {ANCHORED_SIDECAR} for run '{name}' — the run predates "
                "the in-training anchored panel or had anchored_eval_freq=0"
            ),
        }
    rows = list(reversed(rows))
    if limit is not None and limit > 0:
        rows = rows[:limit]
    return {"ok": True, "run": name, "panels": rows, "count": len(rows)}
