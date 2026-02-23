"""Generated/frozen script promotion helpers and manifest management."""

from __future__ import annotations

import hashlib
import json
import re
import shutil
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable


class ScriptPromotionError(RuntimeError):
    """Promotion validation or file operation failure."""


SCRIPT_FILENAME_RE = re.compile(r"^[a-z0-9]+_[0-9]{3}(?:_token)?\.py$")


def _utcnow_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def scripts_root() -> Path:
    return Path(__file__).resolve().parent / "scripts"


def frozen_manifest_path() -> Path:
    return scripts_root() / "_frozen_manifest.json"


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _ensure_manifest_shape(raw: dict | None) -> dict:
    if not isinstance(raw, dict):
        raw = {}
    raw.setdefault("version", 1)
    raw.setdefault("updated_at", _utcnow_iso())
    raw.setdefault("cards", {})
    return raw


def load_frozen_manifest(path: Path | None = None) -> dict:
    manifest_path = path or frozen_manifest_path()
    if not manifest_path.exists():
        return _ensure_manifest_shape(None)
    with manifest_path.open("r", encoding="utf-8") as f:
        return _ensure_manifest_shape(json.load(f))


def save_frozen_manifest(manifest: dict, path: Path | None = None) -> None:
    manifest_path = path or frozen_manifest_path()
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest["updated_at"] = _utcnow_iso()
    with manifest_path.open("w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, sort_keys=True)
        f.write("\n")


def _iter_frozen_scripts(root: Path | None = None) -> Iterable[Path]:
    scripts_dir = root or scripts_root()
    for set_dir in sorted(scripts_dir.iterdir()):
        if not set_dir.is_dir():
            continue
        if set_dir.name in {"generated", "__pycache__"}:
            continue
        for py_file in sorted(set_dir.glob("*.py")):
            if py_file.name == "__init__.py":
                continue
            if not SCRIPT_FILENAME_RE.match(py_file.name):
                continue
            yield py_file


def _card_id_from_module_name(module_name: str) -> str:
    base = module_name.removesuffix(".py")
    parts = base.split("_")
    if len(parts) < 2:
        return base.upper()
    prefix = parts[0].upper()
    suffix = "-".join(part.upper() for part in parts[1:])
    return f"{prefix}-{suffix}"


def bootstrap_manifest_from_frozen(path: Path | None = None) -> dict:
    manifest = _ensure_manifest_shape(None)
    cards: Dict[str, dict] = {}
    for frozen_file in _iter_frozen_scripts():
        set_id = frozen_file.parent.name
        module_name = frozen_file.stem
        card_id = _card_id_from_module_name(frozen_file.name)
        frozen_relpath = str(frozen_file.relative_to(scripts_root())).replace("\\", "/")
        cards[card_id] = {
            "card_id": card_id,
            "set_id": set_id,
            "module_name": module_name,
            "frozen_relpath": frozen_relpath,
            "frozen_hash": _sha256(frozen_file),
            "generated_relpath": f"generated/{set_id}/{frozen_file.name}",
            "generated_hash": None,
            "promoted_at": None,
        }

    manifest["cards"] = cards
    save_frozen_manifest(manifest, path=path)
    return manifest


def promote_script_from_generated(
    *,
    card_id: str,
    set_id: str,
    module_name: str,
    expected_generated_hash: str,
    manifest_path: Path | None = None,
) -> dict:
    set_name = set_id.lower()
    module_filename = f"{module_name}.py"

    root = scripts_root()
    generated_file = root / "generated" / set_name / module_filename
    frozen_file = root / set_name / module_filename

    if not generated_file.exists():
        raise ScriptPromotionError(f"Generated script not found: {generated_file}")

    actual_generated_hash = _sha256(generated_file)
    if actual_generated_hash != expected_generated_hash:
        raise ScriptPromotionError(
            "Generated hash mismatch: expected "
            f"{expected_generated_hash}, got {actual_generated_hash}"
        )

    frozen_file.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(generated_file, frozen_file)
    frozen_hash = _sha256(frozen_file)

    manifest = load_frozen_manifest(path=manifest_path)
    manifest["version"] = int(manifest.get("version", 0)) + 1
    manifest.setdefault("cards", {})
    manifest["cards"][card_id] = {
        "card_id": card_id,
        "set_id": set_name,
        "module_name": module_name,
        "frozen_relpath": str(frozen_file.relative_to(root)).replace("\\", "/"),
        "generated_relpath": str(generated_file.relative_to(root)).replace("\\", "/"),
        "frozen_hash": frozen_hash,
        "generated_hash": actual_generated_hash,
        "promoted_at": _utcnow_iso(),
    }
    save_frozen_manifest(manifest, path=manifest_path)

    return {
        "card_id": card_id,
        "set_id": set_name,
        "module_name": module_name,
        "generated_hash": actual_generated_hash,
        "frozen_hash": frozen_hash,
        "manifest_version": int(manifest["version"]),
    }

