"""``run_checkpoints`` handler.

Checkpoint files match ``code/digimon_gym/agents/pilot_training.py:593``::

    ckpt_dir / f"step_{self.num_timesteps:09d}.zip"

— 9-digit zero-padded step. Sort by step ascending.
"""

from __future__ import annotations

import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

from .paths import resolve_model_run_dir


CHECKPOINT_RE = re.compile(r"^step_(?P<step>\d{9})\.zip$")


def run_checkpoints(models_dir: Optional[Path], name: str) -> Dict[str, Any]:
    resolved = resolve_model_run_dir(models_dir, name)
    if resolved is None:
        return {"ok": False, "error": f"no model artifacts found for run '{name}'"}
    run_dir, model_run_id = resolved
    ckpt_dir = run_dir / "checkpoints"
    if not ckpt_dir.is_dir():
        return {"ok": True, "model_run_id": model_run_id, "checkpoints": []}

    entries: List[Dict[str, Any]] = []
    for path in ckpt_dir.glob("step_*.zip"):
        m = CHECKPOINT_RE.match(path.name)
        if not m:
            continue
        try:
            stat = path.stat()
        except OSError:
            continue
        entries.append({
            "step": int(m.group("step")),
            "path": str(path),
            "mtime": stat.st_mtime,
            "mtime_iso": datetime.fromtimestamp(stat.st_mtime, tz=timezone.utc).isoformat(timespec="seconds"),
            "size_mb": round(stat.st_size / (1024 * 1024), 3),
        })

    entries.sort(key=lambda e: e["step"])
    return {
        "ok": True,
        "model_run_id": model_run_id,
        "count": len(entries),
        "checkpoints": entries,
    }
