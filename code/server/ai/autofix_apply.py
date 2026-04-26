"""Profile-aware apply engine for script_autofix task results."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any


PROJECT_ROOT = Path(__file__).resolve().parents[2]
# After the Phase-6 hoist, all Python source lives under code/ inside the repo.
# When the AI pipeline checks out a worktree it gets the repo root, so paths
# returned by the AI (e.g. "digimon_gym/engine/data/scripts/bt13/bt13_006.py")
# must be prefixed with WORKTREE_SOURCE_DIR before resolving against the worktree root.
WORKTREE_SOURCE_DIR = "code"
MAX_CONTEXT_CHARS_PER_FILE = int(os.environ.get("AI_AUTOFIX_CONTEXT_CHARS_PER_FILE", "8000"))
MAX_CONTEXT_CHARS_TOTAL = int(os.environ.get("AI_AUTOFIX_CONTEXT_CHARS_TOTAL", "16000"))

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
SCOPE_VALUES = {
    SCOPE_SCRIPT,
    SCOPE_SCRIPT_ENGINE,
}

# Curated engine files loaded for script_engine (and wider) scopes.
ENGINE_CORE_FILES = [
    "digimon_gym/engine/core/card_script.py",
    "digimon_gym/engine/core/permanent.py",
    "digimon_gym/engine/interfaces/card_effect.py",
]

class ApplyValidationError(RuntimeError):
    """Raised when fix output violates scope/hash safety rules."""


class HashMismatchError(ApplyValidationError):
    """Raised when expected file hash differs from the current file hash."""

    def __init__(
        self,
        *,
        path: str,
        expected_hash: str,
        current_hash: str,
        can_force_apply: bool,
    ) -> None:
        super().__init__(
            f"Hash mismatch for {path}: expected {expected_hash} got {current_hash}"
        )
        self.path = path
        self.expected_hash = expected_hash
        self.current_hash = current_hash
        self.can_force_apply = can_force_apply


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
    # LLM may return scripts/generated/{set_id}/ — redirect to frozen lane
    normalized = normalized.replace(
        "digimon_gym/engine/data/scripts/generated/",
        "digimon_gym/engine/data/scripts/",
    )
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
    primary = f"digimon_gym/engine/data/scripts/{set_id}/{module_name}.py"
    if rel_path == primary:
        return True
    if scope_profile == SCOPE_SCRIPT:
        return False
    if rel_path.endswith(".py") and rel_path.startswith("digimon_gym/engine/"):
        return True
    return False


def get_allowed_roots_for_scope(scope_profile: str) -> list[str]:
    roots = ["digimon_gym/engine/data/scripts/"]
    if scope_profile == SCOPE_SCRIPT_ENGINE:
        roots.append("digimon_gym/engine/")
    return roots


def get_primary_script_path(set_id: str, module_name: str) -> str:
    return f"digimon_gym/engine/data/scripts/{set_id}/{module_name}.py"


def _read_text_with_cap(path: Path, max_chars: int) -> tuple[str, bool]:
    raw = path.read_text(encoding="utf-8")
    if max_chars <= 0 or len(raw) <= max_chars:
        return raw, False
    clipped = raw[:max_chars]
    suffix = f"\n# [context truncated: showing first {max_chars} chars]\n"
    return clipped + suffix, True


def build_file_context_for_task(*, set_id: str, module_name: str, scope_profile: str) -> list[dict[str, str]]:
    paths = [get_primary_script_path(set_id, module_name)]

    if scope_profile == SCOPE_SCRIPT_ENGINE:
        paths.extend(ENGINE_CORE_FILES)

    contexts: list[dict[str, str]] = []
    remaining_chars = MAX_CONTEXT_CHARS_TOTAL
    for rel in paths:
        if remaining_chars <= 0:
            break
        path = PROJECT_ROOT / rel
        if not path.exists():
            continue
        per_file_budget = min(MAX_CONTEXT_CHARS_PER_FILE, remaining_chars)
        content, truncated = _read_text_with_cap(path, per_file_budget)
        contexts.append(
            {
                "path": rel,
                "hash": _sha256(path),
                "content": content,
                "content_truncated": "true" if truncated else "false",
            }
        )
        remaining_chars -= len(content)
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
    force: bool = False,
    force_allowed_path: str | None = None,
    worktree: bool = False,
) -> list[str]:
    """Apply validated edits to files under repo_root.

    When ``worktree=True`` the edits are being applied to an external git
    worktree checked out from the Phase-6 layout (source under ``code/``).
    In that case each logical path like ``"digimon_gym/engine/data/scripts/…"``
    is resolved as ``repo_root / WORKTREE_SOURCE_DIR / logical_path`` and the
    returned ``applied_files`` list carries the full worktree-relative path
    (i.e. ``"code/digimon_gym/engine/data/scripts/…"``).
    """
    if force:
        if len(edits) != 1:
            raise ApplyValidationError(
                "Force apply is only supported for single-file script edits."
            )
        if not force_allowed_path:
            raise ApplyValidationError(
                "Force apply requires an explicit force_allowed_path."
            )

    applied_paths: list[str] = []
    for edit in edits:
        logical_path = edit["path"]
        if worktree:
            wt_rel = f"{WORKTREE_SOURCE_DIR}/{logical_path}"
            abs_path = repo_root / wt_rel
        else:
            wt_rel = logical_path
            abs_path = repo_root / logical_path
        if not abs_path.exists():
            raise ApplyValidationError(f"Cannot edit missing file: {logical_path}")
        current_hash = _sha256(abs_path).lower()
        if current_hash != edit["expected_hash"]:
            can_force_apply = (
                len(edits) == 1
                and force_allowed_path is not None
                and logical_path == force_allowed_path
            )
            if force and can_force_apply:
                pass
            else:
                raise HashMismatchError(
                    path=logical_path,
                    expected_hash=edit["expected_hash"],
                    current_hash=current_hash,
                    can_force_apply=can_force_apply,
                )
        abs_path.write_text(edit["new_content"], encoding="utf-8")
        applied_paths.append(wt_rel)
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


def derive_targeted_tests(applied_files: list[str], *, worktree: bool = False) -> list[str]:
    """Derive a list of test paths to run for the given applied files.

    ``applied_files`` may contain either plain logical paths
    (``"digimon_gym/engine/…"``) or Phase-6 worktree-relative paths
    (``"code/digimon_gym/engine/…"``).  Both forms are handled automatically.

    When ``worktree=True`` the returned paths carry the ``code/`` prefix so
    they are correct relative to the external worktree root.  When
    ``worktree=False`` they are plain paths relative to ``PROJECT_ROOT``
    (i.e. the ``code/`` directory itself).
    """
    tests: list[str] = []
    touched_sets: set[str] = set()
    has_engine_change = False

    _scripts_prefix = "digimon_gym/engine/data/scripts/"
    _engine_prefix = "digimon_gym/engine/"
    _engine_prefix_wt = f"{WORKTREE_SOURCE_DIR}/{_engine_prefix}"

    for rel in applied_files:
        # Strip optional worktree prefix so we can use a single code path.
        logical = rel[len(WORKTREE_SOURCE_DIR) + 1:] if rel.startswith(_engine_prefix_wt) else rel
        if logical.startswith(_scripts_prefix) and not logical.startswith(_scripts_prefix + "_"):
            parts = logical.split("/")
            # parts: ["digimon_gym","engine","data","scripts",<set_id>,<file>]
            if len(parts) >= 6:
                touched_sets.add(parts[4].lower())
        if logical.startswith(_engine_prefix):
            has_engine_change = True

    for set_id in sorted(touched_sets):
        candidate = f"tests/test_{set_id}_scripts.py"
        if (PROJECT_ROOT / candidate).exists():
            tests.append(f"{WORKTREE_SOURCE_DIR}/{candidate}" if worktree else candidate)

    if has_engine_change:
        candidate = "tests/test_ai_pipeline.py"
        if (PROJECT_ROOT / candidate).exists():
            tests.append(f"{WORKTREE_SOURCE_DIR}/{candidate}" if worktree else candidate)

    seen: set[str] = set()
    unique: list[str] = []
    for item in tests:
        if item in seen:
            continue
        seen.add(item)
        unique.append(item)
    return unique


def run_profile_checks(
    *, repo_root: Path, scope_profile: str, applied_files: list[str], worktree: bool = False
) -> list[str]:
    """Run syntax + integrity + targeted pytest checks.

    When ``worktree=True`` the ``repo_root`` is an external git worktree
    checked out from the Phase-6 repo layout (source under ``code/``), and
    ``applied_files`` carry worktree-relative paths with the ``code/`` prefix
    (form returned by :func:`apply_validated_edits` with ``worktree=True``).

    When ``worktree=False`` (default) the ``repo_root`` is the local server
    codebase root (i.e. the ``code/`` directory itself), and ``applied_files``
    use plain logical paths such as ``"digimon_gym/engine/data/scripts/…"``.
    """
    outputs: list[str] = []
    if applied_files:
        compile_targets = [str(repo_root / rel) for rel in applied_files if rel.endswith(".py")]
        if compile_targets:
            outputs.append(_run_check_command(["python", "-m", "py_compile", *compile_targets], cwd=repo_root))

    # After the Phase-6 hoist, the script lives under code/tools/ inside the
    # repo.  When operating against a worktree (repo root) we must prefix with
    # WORKTREE_SOURCE_DIR; when operating against PROJECT_ROOT (= code/) the
    # script is directly at tools/check_frozen_integrity.py.
    integrity_script = (
        f"{WORKTREE_SOURCE_DIR}/tools/check_frozen_integrity.py"
        if worktree
        else "tools/check_frozen_integrity.py"
    )
    outputs.append(_run_check_command(["python", integrity_script], cwd=repo_root))

    tests = derive_targeted_tests(applied_files, worktree=worktree)
    if tests:
        outputs.append(_run_check_command(["python", "-m", "pytest", "-q", *tests], cwd=repo_root))

    return outputs


def check_edit_applicability(
    *,
    result_payload: dict[str, Any],
    scope_profile: str,
    set_id: str,
    module_name: str,
    repo_root: Path | None = None,
) -> str:
    """Read-only check whether a task's edits can still be applied.

    Returns one of:
      "applicable" — all file hashes match, edits can be applied
      "stale"      — at least one file hash has changed
      "invalid"    — result payload fails structural validation
      "not_applicable" — not a script_autofix task or no edits
    """
    if repo_root is None:
        repo_root = PROJECT_ROOT

    if not result_payload or not isinstance(result_payload, dict):
        return "not_applicable"

    edits_raw = result_payload.get("edits")
    if not edits_raw or not isinstance(edits_raw, list) or len(edits_raw) == 0:
        return "not_applicable"

    try:
        validated = validate_edit_payload(
            result_payload=result_payload,
            scope_profile=scope_profile,
            set_id=set_id,
            module_name=module_name,
        )
    except ApplyValidationError:
        return "invalid"

    if not validated:
        return "not_applicable"

    for edit in validated:
        abs_path = repo_root / edit["path"]
        if not abs_path.exists():
            return "stale"
        current_hash = _sha256(abs_path).lower()
        if current_hash != edit["expected_hash"]:
            return "stale"

    return "applicable"


def serialize_check_outputs(check_outputs: list[str]) -> str:
    return json.dumps(check_outputs, ensure_ascii=True)
