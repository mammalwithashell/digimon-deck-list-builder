"""``list_runs`` handler.

Each directory directly under ``--runs-dir`` is one logical run. Per spec, v1
does NOT recurse into nested subdirectories. The MCP keys runs by the
directory name only — model-side tools (recordings/checkpoints/deck_pool)
discover the matching ``<models-dir>/<name>/`` independently.
"""

from __future__ import annotations

import json
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional


ACTIVE_WINDOW_SECONDS = 60.0


def _newest_tfevents_mtime(run_dir: Path) -> Optional[float]:
    """mtime of the most recently written ``events.out.tfevents.*`` file
    under any ``MaskablePPO_*/`` (or ``MaskableRecurrentPPO_*/``) subdirectory,
    or None if none exist."""
    newest: Optional[float] = None
    for tfevents in run_dir.rglob("events.out.tfevents.*"):
        try:
            mtime = tfevents.stat().st_mtime
        except OSError:
            continue
        if newest is None or mtime > newest:
            newest = mtime
    return newest


def _latest_eval_row(evals_jsonl: Path) -> Optional[Dict[str, Any]]:
    """Return the last well-formed JSON row of ``evals.jsonl``, or None.

    Tolerates partial writes (the last line may be truncated mid-row during
    an active run): walks lines in order and tracks the most recent parseable
    object.
    """
    if not evals_jsonl.is_file():
        return None
    latest: Optional[Dict[str, Any]] = None
    try:
        with evals_jsonl.open("r", encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(row, dict):
                    latest = row
    except OSError:
        return None
    return latest


def _started_at(console_log: Path, fallback: Optional[float]) -> Optional[str]:
    """Best-effort ISO timestamp for when the run started.

    Uses ``console.log``'s ctime when available (created-at); falls back to
    ``fallback`` (typically the directory's own mtime).
    """
    candidate_ts: Optional[float]
    if console_log.is_file():
        try:
            candidate_ts = console_log.stat().st_ctime
        except OSError:
            candidate_ts = fallback
    else:
        candidate_ts = fallback
    if candidate_ts is None:
        return None
    return datetime.fromtimestamp(candidate_ts, tz=timezone.utc).isoformat(timespec="seconds")


def list_runs(
    runs_dir: Optional[Path],
    models_dir: Optional[Path] = None,
) -> Dict[str, Any]:
    """List every direct subdirectory of ``runs_dir`` as a logical run."""
    from .per_game import has_eval_game_log

    if runs_dir is None or not runs_dir.is_dir():
        return {"ok": False, "error": "runs_dir not found — pass --runs-dir or run from a directory whose ancestor contains ./runs"}

    now = time.time()
    entries: List[Dict[str, Any]] = []
    for child in runs_dir.iterdir():
        if not child.is_dir():
            continue
        console_log = child / "console.log"
        evals_jsonl = child / "evals.jsonl"

        console_mtime: Optional[float] = None
        if console_log.is_file():
            try:
                console_mtime = console_log.stat().st_mtime
            except OSError:
                console_mtime = None
        tfevents_mtime = _newest_tfevents_mtime(child)

        try:
            dir_mtime = child.stat().st_mtime
        except OSError:
            dir_mtime = 0.0
        last_modified = max(filter(None, (console_mtime, tfevents_mtime, dir_mtime)))

        active = any(
            mt is not None and (now - mt) < ACTIVE_WINDOW_SECONDS
            for mt in (console_mtime, tfevents_mtime)
        )

        latest_row = _latest_eval_row(evals_jsonl)
        latest_step = int(latest_row["step"]) if latest_row and "step" in latest_row else None
        latest_win_rate = (
            float(latest_row["win_rate"]) if latest_row and "win_rate" in latest_row else None
        )

        entries.append({
            "name": child.name,
            "started_at": _started_at(console_log, fallback=dir_mtime),
            "last_modified": datetime.fromtimestamp(last_modified, tz=timezone.utc).isoformat(timespec="seconds"),
            "last_modified_epoch": last_modified,
            "active": active,
            "latest_step": latest_step,
            "latest_win_rate": latest_win_rate,
            "has_eval_game_log": has_eval_game_log(models_dir, child.name),
        })

    entries.sort(key=lambda e: e["last_modified_epoch"], reverse=True)
    return {"ok": True, "runs_dir": str(runs_dir), "runs": entries}
