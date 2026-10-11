"""Guard ordered OpenSpec archives and dependent revalidation.

OpenSpec validates one change at a time but does not model dependencies between
active changes.  This module supplies the repository-owned ordering contract
for overlapping card-data changes.  A dependent revalidation receipt is bound
to both the archived prerequisite and the exact dependent delta, so changing
either side requires another strict validation before the dependent can be
archived.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any, Callable, Sequence

from .serialization import canonical_json_text


CONFIG_RELATIVE_PATH = Path("openspec/archive-order.json")
RECEIPT_SCHEMA_VERSION = 1
_CHANGE_NAME_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


class ArchiveGuardError(RuntimeError):
    """A stable, operator-facing archive guard failure."""

    def __init__(
        self,
        code: str,
        message: str,
        *,
        details: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or {}

    def as_dict(self) -> dict[str, Any]:
        return {
            "code": self.code,
            "message": self.message,
            "details": self.details,
        }


@dataclass(frozen=True)
class ArchiveConstraint:
    id: str
    prerequisite: str
    dependent: str
    require_dependent_revalidation: bool
    revalidation_receipt: Path


@dataclass(frozen=True)
class ChangeLocation:
    name: str
    state: str
    active_path: Path | None
    archive_paths: tuple[Path, ...]

    @property
    def selected_path(self) -> Path | None:
        if self.state == "active":
            return self.active_path
        if self.state == "archived" and len(self.archive_paths) == 1:
            return self.archive_paths[0]
        return None


CommandRunner = Callable[
    [Sequence[str], Path], subprocess.CompletedProcess[str]
]


def _default_runner(
    command: Sequence[str], cwd: Path
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def _require_relative_repo_path(value: Any, *, field: str) -> Path:
    if not isinstance(value, str) or not value:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CONFIG_INVALID",
            f"{field} must be a non-empty repository-relative path",
        )
    pure = PurePosixPath(value)
    if pure.is_absolute() or ".." in pure.parts:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CONFIG_INVALID",
            f"{field} must stay within the repository",
            details={"value": value},
        )
    return Path(*pure.parts)


def load_constraints(repo_root: Path) -> tuple[ArchiveConstraint, ...]:
    root = repo_root.resolve()
    config_path = root / CONFIG_RELATIVE_PATH
    try:
        payload = json.loads(config_path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CONFIG_MISSING",
            f"archive-order configuration is missing: {CONFIG_RELATIVE_PATH.as_posix()}",
        ) from exc
    except json.JSONDecodeError as exc:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CONFIG_INVALID",
            f"archive-order configuration is not valid JSON: {exc}",
        ) from exc

    if payload.get("schema_version") != 1:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CONFIG_INVALID",
            "archive-order configuration must use schema_version 1",
        )
    raw_constraints = payload.get("constraints")
    if not isinstance(raw_constraints, list) or not raw_constraints:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CONFIG_INVALID",
            "archive-order configuration must declare at least one constraint",
        )

    constraints: list[ArchiveConstraint] = []
    seen_ids: set[str] = set()
    seen_dependents: set[str] = set()
    for raw in raw_constraints:
        if not isinstance(raw, dict):
            raise ArchiveGuardError(
                "OPENSPEC_ARCHIVE_CONFIG_INVALID",
                "each archive-order constraint must be an object",
            )
        constraint_id = raw.get("id")
        prerequisite = raw.get("prerequisite")
        dependent = raw.get("dependent")
        for field, value in (
            ("id", constraint_id),
            ("prerequisite", prerequisite),
            ("dependent", dependent),
        ):
            if not isinstance(value, str) or not _CHANGE_NAME_RE.fullmatch(value):
                raise ArchiveGuardError(
                    "OPENSPEC_ARCHIVE_CONFIG_INVALID",
                    f"constraint {field} must be a lowercase OpenSpec identifier",
                    details={"field": field, "value": value},
                )
        if constraint_id in seen_ids or dependent in seen_dependents:
            raise ArchiveGuardError(
                "OPENSPEC_ARCHIVE_CONFIG_INVALID",
                "constraint IDs and dependent changes must be unique",
                details={"id": constraint_id, "dependent": dependent},
            )
        if prerequisite == dependent:
            raise ArchiveGuardError(
                "OPENSPEC_ARCHIVE_CONFIG_INVALID",
                "an OpenSpec change cannot depend on itself",
                details={"id": constraint_id},
            )
        if raw.get("require_dependent_revalidation") is not True:
            raise ArchiveGuardError(
                "OPENSPEC_ARCHIVE_CONFIG_INVALID",
                "dependent revalidation must be explicitly required",
                details={"id": constraint_id},
            )
        receipt = _require_relative_repo_path(
            raw.get("revalidation_receipt"), field="revalidation_receipt"
        )
        if receipt.parts[:2] != ("openspec", "archive-revalidations"):
            raise ArchiveGuardError(
                "OPENSPEC_ARCHIVE_CONFIG_INVALID",
                "revalidation receipts must live under openspec/archive-revalidations",
                details={"path": receipt.as_posix()},
            )
        constraints.append(
            ArchiveConstraint(
                id=constraint_id,
                prerequisite=prerequisite,
                dependent=dependent,
                require_dependent_revalidation=True,
                revalidation_receipt=receipt,
            )
        )
        seen_ids.add(constraint_id)
        seen_dependents.add(dependent)
    return tuple(constraints)


def locate_change(repo_root: Path, name: str) -> ChangeLocation:
    root = repo_root.resolve()
    active_path = root / "openspec/changes" / name
    archive_root = root / "openspec/changes/archive"
    archive_pattern = re.compile(
        rf"^\d{{4}}-\d{{2}}-\d{{2}}-{re.escape(name)}$"
    )
    archive_paths = tuple(
        sorted(
            (
                path
                for path in archive_root.iterdir()
                if path.is_dir() and archive_pattern.fullmatch(path.name)
            ),
            key=lambda path: path.name,
        )
    ) if archive_root.is_dir() else ()
    active = active_path.is_dir()
    if active and not archive_paths:
        state = "active"
    elif not active and len(archive_paths) == 1:
        state = "archived"
    elif not active and not archive_paths:
        state = "missing"
    else:
        state = "ambiguous"
    return ChangeLocation(
        name=name,
        state=state,
        active_path=active_path if active else None,
        archive_paths=archive_paths,
    )


def _tree_digest(path: Path) -> str:
    if not path.is_dir():
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CHANGE_MISSING",
            f"cannot digest missing OpenSpec change directory: {path}",
        )
    digest = hashlib.sha256()
    digest.update(b"openspec-change-tree-v1\0")
    files = sorted(
        (candidate for candidate in path.rglob("*") if candidate.is_file()),
        key=lambda candidate: candidate.relative_to(path).as_posix(),
    )
    for candidate in files:
        relative = candidate.relative_to(path).as_posix().encode("utf-8")
        content_digest = hashlib.sha256(candidate.read_bytes()).digest()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(content_digest)
    return digest.hexdigest()


def _strict_validation_command(change_name: str) -> list[str]:
    return [
        "openspec",
        "validate",
        change_name,
        "--type",
        "change",
        "--strict",
        "--json",
        "--no-interactive",
    ]


def run_strict_validation(
    repo_root: Path,
    change_name: str,
    *,
    runner: CommandRunner = _default_runner,
) -> dict[str, Any]:
    root = repo_root.resolve()
    command = _strict_validation_command(change_name)
    completed = runner(command, root)
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise ArchiveGuardError(
            "OPENSPEC_DEPENDENT_REVALIDATION_FAILED",
            f"strict validation for {change_name} did not return valid JSON",
            details={
                "returncode": completed.returncode,
                "stderr": completed.stderr.strip(),
            },
        ) from exc
    items = payload.get("items") if isinstance(payload, dict) else None
    matching = [
        item
        for item in items or []
        if isinstance(item, dict)
        and item.get("id") == change_name
        and item.get("type") == "change"
    ]
    if (
        completed.returncode != 0
        or len(matching) != 1
        or matching[0].get("valid") is not True
    ):
        raise ArchiveGuardError(
            "OPENSPEC_DEPENDENT_REVALIDATION_FAILED",
            f"strict OpenSpec validation failed for {change_name}",
            details={
                "returncode": completed.returncode,
                "issues": matching[0].get("issues", []) if matching else [],
                "stderr": completed.stderr.strip(),
            },
        )
    return payload


def _constraint_for_dependent(
    constraints: Sequence[ArchiveConstraint], dependent: str
) -> ArchiveConstraint:
    matches = [item for item in constraints if item.dependent == dependent]
    if len(matches) != 1:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CHANGE_UNGUARDED",
            f"{dependent} is not the dependent in exactly one archive-order constraint",
        )
    return matches[0]


def _relative_path(repo_root: Path, path: Path) -> str:
    return path.resolve().relative_to(repo_root.resolve()).as_posix()


def write_revalidation_receipt(
    repo_root: Path,
    dependent: str,
    *,
    runner: CommandRunner = _default_runner,
) -> dict[str, Any]:
    root = repo_root.resolve()
    constraint = _constraint_for_dependent(load_constraints(root), dependent)
    prerequisite_location = locate_change(root, constraint.prerequisite)
    dependent_location = locate_change(root, dependent)
    if prerequisite_location.state != "archived":
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_PREREQUISITE_ACTIVE",
            f"archive {constraint.prerequisite} before revalidating {dependent}",
            details={"prerequisite_state": prerequisite_location.state},
        )
    if dependent_location.state != "active":
        raise ArchiveGuardError(
            "OPENSPEC_DEPENDENT_NOT_ACTIVE",
            f"{dependent} must still be active when its revalidation receipt is created",
            details={"dependent_state": dependent_location.state},
        )

    validation = run_strict_validation(root, dependent, runner=runner)
    prerequisite_path = prerequisite_location.selected_path
    dependent_path = dependent_location.selected_path
    assert prerequisite_path is not None
    assert dependent_path is not None
    receipt = {
        "schema_version": RECEIPT_SCHEMA_VERSION,
        "constraint_id": constraint.id,
        "prerequisite": constraint.prerequisite,
        "dependent": constraint.dependent,
        "prerequisite_archive_path": _relative_path(root, prerequisite_path),
        "prerequisite_archive_digest": _tree_digest(prerequisite_path),
        "dependent_change_digest": _tree_digest(dependent_path),
        "validation_command": _strict_validation_command(dependent),
        "validation_result_digest": hashlib.sha256(
            canonical_json_text(validation).encode("utf-8")
        ).hexdigest(),
    }
    receipt_path = root / constraint.revalidation_receipt
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    receipt_path.write_text(
        canonical_json_text(receipt), encoding="utf-8", newline="\n"
    )
    return receipt


def validate_revalidation_receipt(
    repo_root: Path,
    constraint: ArchiveConstraint,
) -> dict[str, Any]:
    root = repo_root.resolve()
    receipt_path = root / constraint.revalidation_receipt
    try:
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ArchiveGuardError(
            "OPENSPEC_REVALIDATION_RECEIPT_MISSING",
            f"revalidate {constraint.dependent} after archiving {constraint.prerequisite}",
            details={"receipt": constraint.revalidation_receipt.as_posix()},
        ) from exc
    except json.JSONDecodeError as exc:
        raise ArchiveGuardError(
            "OPENSPEC_REVALIDATION_RECEIPT_INVALID",
            f"revalidation receipt is not valid JSON: {constraint.revalidation_receipt.as_posix()}",
        ) from exc

    prerequisite_location = locate_change(root, constraint.prerequisite)
    dependent_location = locate_change(root, constraint.dependent)
    if prerequisite_location.state != "archived":
        raise ArchiveGuardError(
            "OPENSPEC_REVALIDATION_RECEIPT_STALE",
            "the prerequisite is not in one unambiguous archive location",
            details={"prerequisite_state": prerequisite_location.state},
        )
    if dependent_location.state not in {"active", "archived"}:
        raise ArchiveGuardError(
            "OPENSPEC_REVALIDATION_RECEIPT_STALE",
            "the dependent is not in one unambiguous active or archive location",
            details={"dependent_state": dependent_location.state},
        )
    prerequisite_path = prerequisite_location.selected_path
    dependent_path = dependent_location.selected_path
    assert prerequisite_path is not None
    assert dependent_path is not None
    expected = {
        "schema_version": RECEIPT_SCHEMA_VERSION,
        "constraint_id": constraint.id,
        "prerequisite": constraint.prerequisite,
        "dependent": constraint.dependent,
        "prerequisite_archive_path": _relative_path(root, prerequisite_path),
        "prerequisite_archive_digest": _tree_digest(prerequisite_path),
        "dependent_change_digest": _tree_digest(dependent_path),
        "validation_command": _strict_validation_command(constraint.dependent),
    }
    mismatches = {
        key: {"expected": value, "actual": receipt.get(key)}
        for key, value in expected.items()
        if receipt.get(key) != value
    }
    if not _SHA256_RE.fullmatch(str(receipt.get("validation_result_digest", ""))):
        mismatches["validation_result_digest"] = {
            "expected": "sha256",
            "actual": receipt.get("validation_result_digest"),
        }
    if mismatches:
        raise ArchiveGuardError(
            "OPENSPEC_REVALIDATION_RECEIPT_STALE",
            f"revalidation receipt for {constraint.dependent} is stale",
            details={"mismatches": mismatches},
        )
    return receipt


def audit_repository(repo_root: Path) -> dict[str, Any]:
    root = repo_root.resolve()
    constraints = load_constraints(root)
    rows: list[dict[str, Any]] = []
    diagnostics: list[dict[str, Any]] = []
    for constraint in constraints:
        prerequisite = locate_change(root, constraint.prerequisite)
        dependent = locate_change(root, constraint.dependent)
        row: dict[str, Any] = {
            "id": constraint.id,
            "prerequisite": constraint.prerequisite,
            "prerequisite_state": prerequisite.state,
            "dependent": constraint.dependent,
            "dependent_state": dependent.state,
            "receipt": constraint.revalidation_receipt.as_posix(),
        }
        if prerequisite.state in {"missing", "ambiguous"}:
            error = ArchiveGuardError(
                "OPENSPEC_ARCHIVE_CHANGE_LOCATION_INVALID",
                f"cannot establish one location for {constraint.prerequisite}",
                details={"state": prerequisite.state},
            )
            diagnostics.append(error.as_dict())
            row["status"] = "invalid"
        elif dependent.state in {"missing", "ambiguous"}:
            error = ArchiveGuardError(
                "OPENSPEC_ARCHIVE_CHANGE_LOCATION_INVALID",
                f"cannot establish one location for {constraint.dependent}",
                details={"state": dependent.state},
            )
            diagnostics.append(error.as_dict())
            row["status"] = "invalid"
        elif prerequisite.state == "active" and dependent.state == "archived":
            error = ArchiveGuardError(
                "OPENSPEC_ARCHIVE_ORDER_VIOLATION",
                f"{constraint.dependent} was archived before {constraint.prerequisite}",
            )
            diagnostics.append(error.as_dict())
            row["status"] = "invalid"
        elif prerequisite.state == "active" and dependent.state == "active":
            receipt_path = root / constraint.revalidation_receipt
            if receipt_path.exists():
                error = ArchiveGuardError(
                    "OPENSPEC_REVALIDATION_RECEIPT_STALE",
                    "a dependent revalidation receipt cannot precede prerequisite archival",
                    details={"receipt": constraint.revalidation_receipt.as_posix()},
                )
                diagnostics.append(error.as_dict())
                row["status"] = "invalid"
            else:
                row["status"] = "waiting_for_prerequisite_archive"
        elif prerequisite.state == "archived" and dependent.state == "active":
            try:
                validate_revalidation_receipt(root, constraint)
            except ArchiveGuardError as error:
                if error.code == "OPENSPEC_REVALIDATION_RECEIPT_MISSING":
                    row["status"] = "dependent_revalidation_required"
                else:
                    diagnostics.append(error.as_dict())
                    row["status"] = "invalid"
            else:
                row["status"] = "dependent_revalidated_ready_to_archive"
        elif prerequisite.state == "archived" and dependent.state == "archived":
            try:
                validate_revalidation_receipt(root, constraint)
            except ArchiveGuardError as error:
                diagnostics.append(error.as_dict())
                row["status"] = "invalid"
            else:
                row["status"] = "archive_order_satisfied"
        else:
            error = ArchiveGuardError(
                "OPENSPEC_ARCHIVE_ORDER_VIOLATION",
                "unsupported archive-order state transition",
                details={
                    "prerequisite_state": prerequisite.state,
                    "dependent_state": dependent.state,
                },
            )
            diagnostics.append(error.as_dict())
            row["status"] = "invalid"
        rows.append(row)
    return {
        "schema_version": 1,
        "valid": not diagnostics,
        "constraints": rows,
        "diagnostics": diagnostics,
    }


def archive_change(
    repo_root: Path,
    change_name: str,
    *,
    runner: CommandRunner = _default_runner,
) -> subprocess.CompletedProcess[str]:
    root = repo_root.resolve()
    constraints = load_constraints(root)
    relevant = [
        item
        for item in constraints
        if change_name in {item.prerequisite, item.dependent}
    ]
    if len(relevant) != 1:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_CHANGE_UNGUARDED",
            f"{change_name} is not governed by exactly one archive-order constraint",
        )
    constraint = relevant[0]
    target = locate_change(root, change_name)
    if target.state != "active":
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_TARGET_NOT_ACTIVE",
            f"{change_name} must be active before it can be archived",
            details={"state": target.state},
        )

    if change_name == constraint.prerequisite:
        dependent = locate_change(root, constraint.dependent)
        if dependent.state != "active":
            raise ArchiveGuardError(
                "OPENSPEC_ARCHIVE_ORDER_VIOLATION",
                f"{constraint.dependent} must remain active while {change_name} archives",
                details={"dependent_state": dependent.state},
            )
    else:
        prerequisite = locate_change(root, constraint.prerequisite)
        if prerequisite.state != "archived":
            raise ArchiveGuardError(
                "OPENSPEC_ARCHIVE_PREREQUISITE_ACTIVE",
                f"archive {constraint.prerequisite} before {change_name}",
                details={"prerequisite_state": prerequisite.state},
            )
        validate_revalidation_receipt(root, constraint)

    # The guard validates immediately before archive. OpenSpec archive then runs
    # its own default validation as a second check; this wrapper intentionally
    # exposes no --no-validate escape hatch.
    run_strict_validation(root, change_name, runner=runner)
    completed = runner(["openspec", "archive", change_name, "--yes"], root)
    if completed.returncode != 0:
        raise ArchiveGuardError(
            "OPENSPEC_ARCHIVE_COMMAND_FAILED",
            f"openspec archive failed for {change_name}",
            details={
                "returncode": completed.returncode,
                "stdout": completed.stdout.strip(),
                "stderr": completed.stderr.strip(),
            },
        )
    return completed


def _print_report(report: dict[str, Any], *, json_output: bool) -> None:
    if json_output:
        print(canonical_json_text(report), end="")
        return
    for row in report.get("constraints", []):
        print(
            f"{row['id']}: {row['status']} "
            f"({row['prerequisite']}={row['prerequisite_state']}, "
            f"{row['dependent']}={row['dependent_state']})"
        )
    for diagnostic in report.get("diagnostics", []):
        print(f"{diagnostic['code']}: {diagnostic['message']}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--json", action="store_true", dest="json_output")
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("check", help="audit current archive order and receipts")
    revalidate_parser = subparsers.add_parser(
        "revalidate", help="strictly validate a dependent and write its receipt"
    )
    revalidate_parser.add_argument("change")
    archive_parser = subparsers.add_parser(
        "archive", help="guard, strictly validate, and archive one governed change"
    )
    archive_parser.add_argument("change")
    args = parser.parse_args(argv)

    try:
        if args.command == "check":
            report = audit_repository(args.repo_root)
            _print_report(report, json_output=args.json_output)
            return 0 if report["valid"] else 1
        if args.command == "revalidate":
            receipt = write_revalidation_receipt(args.repo_root, args.change)
            report = {"valid": True, "receipt": receipt}
            _print_report(report, json_output=args.json_output)
            return 0
        completed = archive_change(args.repo_root, args.change)
        report = {
            "valid": True,
            "archived_change": args.change,
            "openspec_stdout": completed.stdout.strip(),
        }
        _print_report(report, json_output=args.json_output)
        return 0
    except ArchiveGuardError as error:
        report = {"valid": False, "diagnostics": [error.as_dict()]}
        _print_report(report, json_output=args.json_output)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
