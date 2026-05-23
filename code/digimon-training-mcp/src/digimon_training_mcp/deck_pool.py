"""``run_deck_pool`` handler.

Reads ``deck_pool_snapshot.json`` verbatim. The snapshot is produced by
``code/digimon_gym/agents/pilot_training.py:976`` for generalist runs and is
absent for non-gauntlet runs — return a structured error in that case rather
than raising.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Optional

from .paths import resolve_model_run_dir


def run_deck_pool(models_dir: Optional[Path], name: str) -> Dict[str, Any]:
    resolved = resolve_model_run_dir(models_dir, name)
    if resolved is None:
        return {"ok": False, "error": f"no model artifacts found for run '{name}'"}
    run_dir, model_run_id = resolved
    path = run_dir / "deck_pool_snapshot.json"
    if not path.is_file():
        return {
            "ok": False,
            "model_run_id": model_run_id,
            "error": f"deck_pool_snapshot.json not found for run '{name}'",
        }
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return {
            "ok": False,
            "model_run_id": model_run_id,
            "error": f"failed to read deck_pool_snapshot.json: {type(exc).__name__}: {exc}",
        }
    decks = data.get("decks", []) if isinstance(data, dict) else []
    archetypes = data.get("archetypes", []) if isinstance(data, dict) else []
    return {
        "ok": True,
        "model_run_id": model_run_id,
        "archetypes": archetypes,
        "deck_count": len(decks) if isinstance(decks, list) else 0,
        "decks": decks,
    }
