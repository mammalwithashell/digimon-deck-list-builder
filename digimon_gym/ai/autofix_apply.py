"""Profile-aware apply engine for script_autofix task results."""

from __future__ import annotations

import hashlib
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any


PROJECT_ROOT = Path(__file__).resolve().parents[2]

DENY_PREFIXES = (
    ".github/",
    "alembic/",
    ".git/",
    ".claude/",
    "data/",
)
DENY_EXACT = {
    ".env",
    ".env.example",
    "requirements.txt",
    "pyproject.toml",
    "package-lock.json",
}

SCOPE_SCRIPT = "script"
SCOPE_SCRIPT_ENGINE = "script_engine"
SCOPE_SCRIPT_ENGINE_TRANSPILER = "script_engine_transpiler"
SCOPE_VALUES = {
    SCOPE_SCRIPT,
    SCOPE_SCRIPT_ENGINE,
    SCOPE_SCRIPT_ENGINE_TRANSPILER,
}


class ApplyValidationError(RuntimeError):
    """Raised when fix output violates scope/hash safety rules."""


@dataclass
class ApplyResult:
    applied_files: list[str]
    check_outputs: list[str]


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _normalize_rel_path(raw_path: str) -> str:
    candidate = Path(raw_path.replace("\\", "/"))
    if candidate.is_absolute():
        raise ApplyValidationError(f"Absolute path is not allowed: {raw_path}")
    normalized = str(candidate).replace("\\", "/")
    if normalized.startswith("../") or "/../" in normalized or normalized == "..":
        raise ApplyValidationError(f"Path traversal is not allowed: {raw_path}")
    if normalized.startswith("./"):
        normalized = normalized[2:]
    if not normalized:
        raise ApplyValidationError("Empty edit path is not allowed")
    return normalized


def _is_denied_path(rel_path: str) -> bool:
    if rel_path in DENY_EXACT:
        return True
    return any(rel_path.startswith(prefix) for prefix in DENY_PREFIXES)


def _is_allowed_for_scope(
    rel_path: str,
    *,
    scope_profile: str,
    set_id: str,
    module_name: str,
) -> bool:
    primary = f"digimon_gym/engine/data/scripts/generated/{set_id}/{module_name}.py"
    if rel_path == primary:
        return True
    if scope_profile == SCOPE_SCRIPT:
        return False
    if rel_path.endswith(".py") and rel_path.startswith("digimon_gym/engine/"):
        return True
    if scope_profile == SCOPE_SCRIPT_ENGINE_TRANSPILER:
        if rel_path.endswith(".py") and rel_path.startswith("tools/transpiler/"):
            return True
    return False


def get_allowed_roots_for_scope(scope_profile: str) -> list[str]:
    roots = ["digimon_gym/engine/data/scripts/generated/"]
    if scope_profile in {SCOPE_SCRIPT_ENGINE, SCOPE_SCRIPT_ENGINE_TRANSPILER}:
        roots.append("digimon_gym/engine/")
    if scope_profile == SCOPE_SCRIPT_ENGINE_TRANSPILER:
        roots.append("tools/transpiler/")
    return roots


def get_primary_script_path(set_id: str, module_name: str) -> str:
    return f"digimon_gym/engine/data/scripts/generated/{set_id}/{module_name}.py"


def build_file_context_for_task(*, set_id: str, module_name: str, scope_profile: str) -> list[dict[str, str]]:
    paths = [get_primary_script_path(set_id, module_name)]
    contexts: list[dict[str, str]] = []
    for rel in paths:
        path = PROJECT_ROOT / rel
        if not path.exists():
            continue
        contexts.append(
            {
                "path": rel,
                "hash": _sha256(path),
                "content": path.read_text(encoding="utf-8"),
            }
        )
    return contexts


def validate_edit_payload(
    *,
    result_payload: dict[str, Any],
    scope_profile: str,
    set_id: str,
    module_name: str,
) -> list[dict[str, str]]:
    if scope_profile not in SCOPE_VALUES:
        raise ApplyValidationError(f"Unknown scope profile: {scope_profile}")
    edits = result_payload.get("edits", [])
    if not isinstance(edits, list):
        raise ApplyValidationError("Result payload must include 'edits' list")

    validated: list[dict[str, str]] = []
    for entry in edits:
        if not isinstance(entry, dict):
            raise ApplyValidationError("Each edit must be an object")
        raw_path = str(entry.get("path", "")).strip()
        expected_hash = str(entry.get("expected_hash", "")).strip().lower()
        new_content = entry.get("new_content")
        if not raw_path or not expected_hash or not isinstance(new_content, str):
            raise ApplyValidationError("Edit must include path, expected_hash, new_content")

        rel_path = _normalize_rel_path(raw_path)
        if _is_denied_path(rel_path):
            raise ApplyValidationError(f"Path denied by policy: {rel_path}")
        if not _is_allowed_for_scope(
            rel_path,
            scope_profile=scope_profile,
            set_id=set_id,
            module_name=module_name,
        ):
            raise ApplyValidationError(
                f"Edit path outside scope profile '{scope_profile}': {rel_path}"
            )
        validated.append(
            {
                "path": rel_path,
                "expected_hash": expected_hash,
                "new_content": new_content,
            }
        )

    return validated


def apply_validated_edits(
    *,
    repo_root: Path,
    edits: list[dict[str, str]],
) -> list[str]:
    applied_paths: list[str] = []
    for edit in edits:
        abs_path = repo_root / edit["path"]
        if not abs_path.exists():
            raise ApplyValidationError(f"Cannot edit missing file: {edit['path']}")
        current_hash = _sha256(abs_path).lower()
        if current_hash != edit["expected_hash"]:
            raise ApplyValidationError(
                f"Hash mismatch for {edit['path']}: expected {edit['expected_hash']} got {current_hash}"
            )
        abs_path.write_text(edit["new_content"], encoding="utf-8")
        applied_paths.append(edit["path"])
    return applied_paths


def _run_check_command(args: list[str], *, cwd: Path) -> str:
    proc = subprocess.run(
        args,
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )
    output = (proc.stdout or "") + (("\n" + proc.stderr) if proc.stderr else "")
    output = output.strip()
    cmd = " ".join(args)
    if proc.returncode != 0:
        raise ApplyValidationError(f"Check failed ({cmd}): {output or 'no output'}")
    return f"{cmd}: {output or 'ok'}"


def derive_targeted_tests(applied_files: list[str]) -> list[str]:
    tests: list[str] = []
    touched_sets: set[str] = set()
    has_engine_change = False
    has_transpiler_change = False

    for rel in applied_files:
        if rel.startswith("digimon_gym/engine/data/scripts/generated/"):
            parts = rel.split("/")
            if len(parts) >= 6:
                touched_sets.add(parts[4].lower())
        if rel.startswith("digimon_gym/engine/"):
            has_engine_change = True
        if rel.startswith("tools/transpiler/"):
            has_transpiler_change = True

    for set_id in sorted(touched_sets):
        candidate = f"tests/test_{set_id}_scripts.py"
        if (PROJECT_ROOT / candidate).exists():
            tests.append(candidate)

    if has_engine_change:
        if (PROJECT_ROOT / "tests/test_ai_pipeline.py").exists():
            tests.append("tests/test_ai_pipeline.py")
    if has_transpiler_change:
        for candidate in ("tests/test_build_registry.py", "tests/test_ai_pipeline.py"):
            if (PROJECT_ROOT / candidate).exists():
                tests.append(candidate)

    seen: set[str] = set()
    unique: list[str] = []
    for item in tests:
        if item in seen:
            continue
        seen.add(item)
        unique.append(item)
    return unique


def run_profile_checks(*, repo_root: Path, scope_profile: str, applied_files: list[str]) -> list[str]:
    outputs: list[str] = []
    if applied_files:
        compile_targets = [str(repo_root / rel) for rel in applied_files if rel.endswith(".py")]
        if compile_targets:
            outputs.append(_run_check_command(["python", "-m", "py_compile", *compile_targets], cwd=repo_root))

    outputs.append(_run_check_command(["python", "scripts/check_frozen_integrity.py"], cwd=repo_root))

    if scope_profile in {SCOPE_SCRIPT_ENGINE, SCOPE_SCRIPT_ENGINE_TRANSPILER}:
        tests = derive_targeted_tests(applied_files)
        if tests:
            outputs.append(_run_check_command(["pytest", "-q", *tests], cwd=repo_root))

    return outputs


def serialize_check_outputs(check_outputs: list[str]) -> str:
    return json.dumps(check_outputs, ensure_ascii=True)
