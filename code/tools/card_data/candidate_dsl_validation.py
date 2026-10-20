"""Network-free production-DSL validation for publication candidates.

The canonical candidate is not installed at the repository's fixed data paths
until after a publication plan has been reviewed.  This adapter therefore runs
the shared Rust production validator with explicit candidate canonical and
correction paths, then verifies that the JSON report names the exact bytes that
the publication plan is about to bind.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from collections.abc import Callable, Mapping
from pathlib import Path, PurePosixPath
from typing import Any

from .errors import ValidationError


CANONICAL_ARTIFACT = "data/cards_canonical.json"
CORRECTIONS_ARTIFACT = "data/card_corrections.json"
DSL_LINT_BINARY_ENV = "DIGIMON_DSL_LINT_BIN"
DSL_LINT_CARGO_ENV = "CARGO"

_CommandRunner = Callable[..., subprocess.CompletedProcess[str]]


def _digest_named_inputs(inputs: Mapping[str, bytes]) -> str:
    """Mirror the shared pack-input digest used in the Rust lint report.

    This does not duplicate a validation rule.  It is only the framing hash
    that proves the report checked the same canonical and corrections bytes
    already loaded from the digest-checked publication inventory.
    """

    hasher = hashlib.sha256()
    for raw_name, body in sorted(inputs.items()):
        name = raw_name.replace("\\", "/").encode("utf-8")
        hasher.update(len(name).to_bytes(8, "big"))
        hasher.update(name)
        hasher.update(len(body).to_bytes(8, "big"))
        hasher.update(body)
    return hasher.hexdigest()


def _candidate_path(candidate_dir: Path, relative: str) -> Path:
    path = (candidate_dir / PurePosixPath(relative)).resolve()
    if not path.is_relative_to(candidate_dir.resolve()):
        raise ValidationError(
            "candidate DSL validation input escapes the candidate directory",
            path=path,
        )
    return path


def _dsl_lint_command(
    *,
    repo_root: Path,
    candidate_dir: Path,
) -> tuple[str, ...]:
    cards_root = repo_root / "code/digimon-engine/cards"
    baseline = cards_root / "core_validation_baseline.json"
    if not cards_root.is_dir():
        raise ValidationError(
            "production DSL card root is unavailable for candidate validation",
            path=cards_root,
        )
    if not baseline.is_file():
        raise ValidationError(
            "production DSL debt baseline is unavailable for candidate validation",
            path=baseline,
        )

    explicit_binary = os.environ.get(DSL_LINT_BINARY_ENV)
    if explicit_binary:
        command: list[str] = [explicit_binary]
    else:
        # --offline makes the publication decision independent of upstream
        # availability.  Operators may prebuild the binary and select it with
        # DIGIMON_DSL_LINT_BIN when the local Cargo cache is intentionally lean.
        command = [
            os.environ.get(DSL_LINT_CARGO_ENV, "cargo"),
            "run",
            "--quiet",
            "--locked",
            "--offline",
            "-p",
            "dsl-lint",
            "--",
        ]
    command.extend(
        [
            os.fspath(cards_root),
            "--production-only",
            "--format",
            "json",
            "--enforcement",
            "no-new-debt",
            "--baseline",
            os.fspath(baseline),
            "--canonical",
            os.fspath(_candidate_path(candidate_dir, CANONICAL_ARTIFACT)),
            "--corrections",
            os.fspath(_candidate_path(candidate_dir, CORRECTIONS_ARTIFACT)),
        ]
    )
    return tuple(command)


def _short_output(value: object, *, limit: int = 4000) -> str:
    rendered = "" if value is None else str(value)
    if len(rendered) <= limit:
        return rendered
    return rendered[-limit:]


def _require_mapping(value: object, field: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ValidationError(
            "candidate production DSL validator emitted an invalid JSON report",
            details={"field": field, "expected": "object"},
        )
    return value


def validate_candidate_production_dsl(
    *,
    repo_root: Path,
    candidate_dir: Path,
    artifact_bytes: Mapping[str, bytes],
    command_runner: _CommandRunner = subprocess.run,
) -> Mapping[str, Any]:
    """Run the shared no-new-debt profile against exact candidate inputs.

    ``artifact_bytes`` must come from ``load_candidate_artifacts`` so the
    inventory has already rejected missing, unexpected, or stale files.  The
    Rust report's named-input revision is then checked against those in-memory
    bytes, closing the gap between the files validated and the files bound into
    the publication plan.
    """

    required: dict[str, bytes] = {}
    for relative in (CANONICAL_ARTIFACT, CORRECTIONS_ARTIFACT):
        body = artifact_bytes.get(relative)
        if not isinstance(body, bytes):
            raise ValidationError(
                "publication candidate is missing a DSL validation input",
                path=_candidate_path(candidate_dir, relative),
                details={"artifact_path": relative},
            )
        required[relative] = body
    expected_revision = _digest_named_inputs(
        {
            "canonical": required[CANONICAL_ARTIFACT],
            "corrections": required[CORRECTIONS_ARTIFACT],
        }
    )
    command = _dsl_lint_command(repo_root=repo_root, candidate_dir=candidate_dir)
    try:
        result = command_runner(
            command,
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as exc:
        raise ValidationError(
            "candidate production DSL validator could not be executed",
            path=repo_root,
            details={"error": str(exc), "command": list(command)},
        ) from exc

    try:
        payload = json.loads(result.stdout)
    except (TypeError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "candidate production DSL validator did not emit valid JSON",
            path=candidate_dir,
            details={
                "return_code": result.returncode,
                "stdout": _short_output(result.stdout),
                "stderr": _short_output(result.stderr),
            },
        ) from exc
    report = _require_mapping(payload, "$")
    if report.get("schema_version") != 1:
        raise ValidationError(
            "candidate production DSL validator report schema is unsupported",
            path=candidate_dir,
            details={"schema_version": report.get("schema_version")},
        )
    if report.get("enforcement") != "no-new-debt":
        raise ValidationError(
            "candidate production DSL validator used the wrong enforcement mode",
            path=candidate_dir,
            details={"enforcement": report.get("enforcement")},
        )

    canonical = _require_mapping(report.get("canonical"), "canonical")
    actual_revision = canonical.get("revision")
    if canonical.get("status") != "checked" or actual_revision != expected_revision:
        raise ValidationError(
            "candidate production DSL validation does not bind the reviewed inputs",
            path=candidate_dir,
            details={
                "canonical_status": canonical.get("status"),
                "expected_input_revision": expected_revision,
                "reported_input_revision": actual_revision,
            },
        )

    summary = _require_mapping(report.get("summary"), "summary")
    blocking_count = summary.get("blocking_count")
    critical_count = summary.get("critical_count")
    if (
        not isinstance(blocking_count, int)
        or isinstance(blocking_count, bool)
        or blocking_count < 0
        or not isinstance(critical_count, int)
        or isinstance(critical_count, bool)
        or critical_count < 0
    ):
        raise ValidationError(
            "candidate production DSL validator emitted an invalid summary",
            path=candidate_dir,
            details={
                "blocking_count": blocking_count,
                "critical_count": critical_count,
            },
        )
    diagnostics = report.get("diagnostics")
    if not isinstance(diagnostics, list):
        raise ValidationError(
            "candidate production DSL validator emitted invalid diagnostics",
            path=candidate_dir,
        )
    if result.returncode != 0 or blocking_count or critical_count:
        raise ValidationError(
            "candidate production DSL validation blocks publication planning",
            path=candidate_dir,
            details={
                "return_code": result.returncode,
                "blocking_count": blocking_count,
                "critical_count": critical_count,
                "diagnostic_count": summary.get("diagnostic_count"),
                "by_code": summary.get("by_code"),
                "stderr": _short_output(result.stderr),
            },
        )
    return report
