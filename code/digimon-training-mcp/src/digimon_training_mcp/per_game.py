"""``run_per_game_evals`` handler.

Reads `eval_game_log.jsonl` (written by `WinRateCallback._run_evaluation`
per the `per-game-eval-log` capability), applies optional filters, sorts
by `(step, eval_window_idx, game_idx)`, and returns rows verbatim.

Path resolution mirrors `recordings.py` / `checkpoints.py`: checks
`<models-dir>/<name>/eval_game_log.jsonl` first, then descends into the
most-recently-modified timestamped subdirectory.
"""

from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any, Dict, List, Optional

from .paths import resolve_model_run_dir


logger = logging.getLogger("digimon_training_mcp.per_game")


def _has_log_marker(p: Path) -> bool:
    return (p / "eval_game_log.jsonl").is_file()


def _resolve_log_path(
    models_dir: Optional[Path], name: str
) -> Optional[tuple[Path, str]]:
    """Return ``(eval_game_log_path, model_run_id)`` or ``None``.

    First checks ``<models-dir>/<name>/eval_game_log.jsonl`` directly. If
    absent, descends into the most-recently-modified timestamped child
    subdirectory and looks there. The ``model_run_id`` mirrors what
    ``recordings.py`` / ``checkpoints.py`` would return for the same run.
    """
    if models_dir is None or not models_dir.is_dir():
        return None
    base = models_dir / name
    if not base.is_dir():
        return None
    direct = base / "eval_game_log.jsonl"
    if direct.is_file():
        return (direct, name)
    # Descend into most-recently-modified subdir that has the marker.
    candidates: list[tuple[float, Path]] = []
    for child in base.iterdir():
        if not child.is_dir():
            continue
        if not _has_log_marker(child):
            continue
        try:
            mtime = child.stat().st_mtime
        except OSError:
            continue
        candidates.append((mtime, child))
    if not candidates:
        # Fall back to resolve_model_run_dir so we still return a model_run_id
        # for empty-file responses (matches `run_recordings` semantics).
        resolved = resolve_model_run_dir(models_dir, name)
        if resolved is None:
            return None
        _, model_run_id = resolved
        return None if model_run_id is None else (base / "eval_game_log.jsonl", model_run_id)
    candidates.sort(reverse=True)
    chosen = candidates[0][1]
    return (chosen / "eval_game_log.jsonl", chosen.name)


def _read_rows(path: Path) -> List[Dict[str, Any]]:
    """Read all JSON rows from a JSONL file. Tolerates a partial last line."""
    rows: List[Dict[str, Any]] = []
    try:
        with path.open("r", encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(row, dict):
                    rows.append(row)
    except OSError:
        return []
    return rows


def _passes_filter(row: Dict[str, Any], f: Dict[str, Any]) -> bool:
    """AND-style filter. Unknown keys are ignored (forward-compat)."""
    if "agent_archetype" in f and row.get("agent_archetype") != f["agent_archetype"]:
        return False
    if "opponent_archetype" in f and row.get("opponent_archetype") != f["opponent_archetype"]:
        return False
    if "result" in f and row.get("result") != f["result"]:
        return False
    if "digivolves_agent_min" in f:
        try:
            if int(row.get("digivolves_agent", 0)) < int(f["digivolves_agent_min"]):
                return False
        except (TypeError, ValueError):
            return False
    if "dna_digivolves_agent_min" in f:
        try:
            if int(row.get("dna_digivolves_agent", 0)) < int(f["dna_digivolves_agent_min"]):
                return False
        except (TypeError, ValueError):
            return False
    if "step_min" in f:
        try:
            if int(row.get("step", 0)) < int(f["step_min"]):
                return False
        except (TypeError, ValueError):
            return False
    if "step_max" in f:
        try:
            if int(row.get("step", 0)) > int(f["step_max"]):
                return False
        except (TypeError, ValueError):
            return False
    return True


def _sort_key(row: Dict[str, Any]) -> tuple:
    return (
        int(row.get("step", 0)),
        int(row.get("eval_window_idx", 0)),
        int(row.get("game_idx", 0)),
    )


def run_per_game_evals(
    models_dir: Optional[Path],
    name: str,
    filter_: Optional[Dict[str, Any]] = None,
    limit: Optional[int] = None,
) -> Dict[str, Any]:
    resolved = _resolve_log_path(models_dir, name)
    if resolved is None:
        return {
            "ok": True,
            "model_run_id": None,
            "count": 0,
            "rows": [],
        }
    log_path, model_run_id = resolved
    rows = _read_rows(log_path) if log_path.is_file() else []
    if filter_:
        rows = [r for r in rows if _passes_filter(r, filter_)]
    rows.sort(key=_sort_key)
    if limit is not None and limit > 0:
        rows = rows[:limit]
    return {
        "ok": True,
        "model_run_id": model_run_id,
        "count": len(rows),
        "rows": rows,
    }


def has_eval_game_log(models_dir: Optional[Path], name: str) -> bool:
    """Cheap existence check used by ``list_runs``."""
    resolved = _resolve_log_path(models_dir, name)
    if resolved is None:
        return False
    log_path, _ = resolved
    return log_path.is_file()
