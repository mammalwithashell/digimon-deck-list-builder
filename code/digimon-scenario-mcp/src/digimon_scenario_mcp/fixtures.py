"""Scenario-fixture file operations over `qa/scenarios/` and pure
fixture-editing helpers (assertion append). All testable without a server.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Optional


def _ancestor_with(child: str, start: Optional[Path] = None) -> Optional[Path]:
    here = (start or Path.cwd()).resolve()
    for d in (here, *here.parents):
        if (d / child).is_dir():
            return d / child
    return None


def resolve_scenarios_dir(explicit: Optional[Path] = None) -> Optional[Path]:
    """Resolve `qa/scenarios/`: explicit path, else ancestor-walk for
    `qa/scenarios` from the cwd."""
    if explicit is not None:
        return explicit if explicit.is_dir() else None
    return _ancestor_with("qa/scenarios")


def list_fixtures(scenarios_dir: Optional[Path]) -> dict[str, Any]:
    if scenarios_dir is None or not scenarios_dir.is_dir():
        return {"ok": False, "error": "scenarios_dir not resolved (pass --scenarios-dir)"}
    items = []
    for p in sorted(scenarios_dir.glob("*.json")):
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            items.append({"name": p.stem, "valid": False})
            continue
        items.append(
            {
                "name": p.stem,
                "title": data.get("title", ""),
                "readiness": data.get("readiness", ""),
                "valid": True,
            }
        )
    return {"ok": True, "scenarios_dir": str(scenarios_dir), "fixtures": items}


def load_fixture(scenarios_dir: Optional[Path], name: str) -> dict[str, Any]:
    if scenarios_dir is None or not scenarios_dir.is_dir():
        return {"ok": False, "error": "scenarios_dir not resolved"}
    path = scenarios_dir / f"{name}.json"
    if not path.is_file():
        return {"ok": False, "error": f"fixture '{name}' not found under {scenarios_dir}"}
    try:
        return {"ok": True, "name": name, "fixture": json.loads(path.read_text(encoding="utf-8"))}
    except (json.JSONDecodeError, OSError) as exc:
        return {"ok": False, "error": f"could not read fixture '{name}': {exc}"}


def save_fixture(scenarios_dir: Optional[Path], name: str, fixture: dict[str, Any]) -> dict[str, Any]:
    if scenarios_dir is None or not scenarios_dir.is_dir():
        return {"ok": False, "error": "scenarios_dir not resolved"}
    if not name or "/" in name or "\\" in name or name.startswith("."):
        return {"ok": False, "error": f"invalid fixture name {name!r}"}
    # Stamp the id so the on-disk fixture is self-identifying.
    fixture = {**fixture, "id": fixture.get("id") or name}
    path = scenarios_dir / f"{name}.json"
    try:
        path.write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")
    except OSError as exc:
        return {"ok": False, "error": f"could not write fixture '{name}': {exc}"}
    return {"ok": True, "name": name, "path": str(path)}


def add_assertion(
    fixture: dict[str, Any], assertion: dict[str, Any], bucket: str = "engine"
) -> dict[str, Any]:
    """Return a copy of `fixture` with `assertion` appended to the named
    assertion bucket (`engine` or `ui`). Pure — no I/O."""
    if bucket not in ("engine", "ui"):
        return {"ok": False, "error": f"bucket must be 'engine' or 'ui', got {bucket!r}"}
    out = json.loads(json.dumps(fixture))  # deep copy
    assertions = out.setdefault("assertions", {})
    if not isinstance(assertions, dict):
        return {"ok": False, "error": "fixture 'assertions' is not an object"}
    assertions.setdefault(bucket, []).append(assertion)
    return {"ok": True, "fixture": out}
