"""Read-only readers for model-evaluation artifacts (champion registry, Elo
ladder, exploitability) produced by the eval harness. Parses the JSON files
directly — this package never imports ``digimon_gym`` / ``server`` / binding
crates. Part of add-model-evaluation-harness (Phase 5)."""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Optional


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def champion_standings(models_dir: Optional[Path]) -> Dict[str, Any]:
    """Return the registered champions (name, provenance, profile) from
    ``<models_dir>/champions/registry.json``."""
    if models_dir is None:
        return {"ok": False, "error": "models_dir not resolved"}
    path = Path(models_dir) / "champions" / "registry.json"
    if not path.exists():
        return {"ok": True, "champions": [], "note": f"no registry at {path}"}
    data = _load_json(path)
    return {"ok": True, "champions": data.get("champions", []),
            "count": len(data.get("champions", []))}


def run_elo_ladder(models_dir: Optional[Path], name: str) -> Dict[str, Any]:
    """Return the persisted Elo ladder for a run
    (``<models_dir>/<name>/elo_ladder.json``)."""
    if models_dir is None:
        return {"ok": False, "error": "models_dir not resolved"}
    path = Path(models_dir) / name / "elo_ladder.json"
    if not path.exists():
        return {"ok": False, "error": f"no elo_ladder.json for run '{name}' "
                f"(run code/tools/elo_ladder_cli.py --run <dir> --out {path})"}
    return {"ok": True, "run": name, **_load_json(path)}


def run_exploitability(models_dir: Optional[Path], name: str) -> Dict[str, Any]:
    """Return the persisted exploitability result for a run
    (``<models_dir>/<name>/exploitability.json``). Labeled as a lower bound."""
    if models_dir is None:
        return {"ok": False, "error": "models_dir not resolved"}
    path = Path(models_dir) / name / "exploitability.json"
    if not path.exists():
        return {"ok": False, "error": f"no exploitability.json for run '{name}'"}
    data = _load_json(path)
    data.setdefault("is_lower_bound", True)
    return {"ok": True, "run": name, **data}
