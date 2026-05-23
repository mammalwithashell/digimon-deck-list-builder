"""``run_recordings`` handler.

Files under ``<resolved-model-run-dir>/recordings/*.json`` have a fixed
filename shape (verified against ``code/digimon_gym/agents/training_recording.py:134``)::

    {source_slug}_env_{env_index:03d}_game_{game_index:06d}_{result}_{reason}.json

Examples (real)::

    train_env_000_game_000034_draw_crash.json
    train_env_003_game_000128_win_decked_out.json
    eval_env_001_game_000099_draw_step_limit.json

No step count appears in the filename; the per-step count lives in the file
body under ``outcome.step_count``. Filter semantics follow design.md §Decision 8.
"""

from __future__ import annotations

import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

from .paths import resolve_model_run_dir


FILENAME_RE = re.compile(
    r"^(?P<source>[A-Za-z0-9-]+)"
    r"_env_(?P<env>\d{3})"
    r"_game_(?P<game>\d{6})"
    r"_(?P<result>[A-Za-z0-9-]+?)"
    r"_(?P<reason>[A-Za-z0-9_-]+)"
    r"\.json$"
)


def _parse(path: Path) -> Optional[Dict[str, Any]]:
    m = FILENAME_RE.match(path.name)
    if not m:
        return None
    try:
        stat = path.stat()
    except OSError:
        return None
    return {
        "path": str(path),
        "source": m.group("source"),
        "env": int(m.group("env")),
        "game": int(m.group("game")),
        "result": m.group("result"),
        "reason": m.group("reason"),
        "mtime": stat.st_mtime,
        "mtime_iso": datetime.fromtimestamp(stat.st_mtime, tz=timezone.utc).isoformat(timespec="seconds"),
        "size_bytes": stat.st_size,
    }


def _passes_filter(entry: Dict[str, Any], filter_mode: str) -> bool:
    if filter_mode == "all":
        return True
    if filter_mode == "crash":
        return entry["reason"] == "crash"
    if filter_mode == "draw":
        return entry["result"] == "draw" and entry["reason"] != "crash"
    # Unknown filter — defensive default to include nothing rather than silently
    # showing all recordings.
    return False


def run_recordings(
    models_dir: Optional[Path],
    name: str,
    filter_mode: str = "all",
    limit: Optional[int] = None,
) -> Dict[str, Any]:
    resolved = resolve_model_run_dir(models_dir, name)
    if resolved is None:
        return {"ok": False, "error": f"no model artifacts found for run '{name}'"}
    run_dir, model_run_id = resolved
    rec_dir = run_dir / "recordings"
    if not rec_dir.is_dir():
        return {"ok": True, "model_run_id": model_run_id, "recordings": []}

    entries: List[Dict[str, Any]] = []
    for path in rec_dir.glob("*.json"):
        parsed = _parse(path)
        if parsed is None:
            continue
        if not _passes_filter(parsed, filter_mode):
            continue
        entries.append(parsed)

    entries.sort(key=lambda e: e["mtime"], reverse=True)
    if limit is not None and limit > 0:
        entries = entries[:limit]

    return {
        "ok": True,
        "model_run_id": model_run_id,
        "filter": filter_mode,
        "count": len(entries),
        "recordings": entries,
    }
