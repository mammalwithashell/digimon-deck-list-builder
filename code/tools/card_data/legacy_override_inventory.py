"""Inventory and guard the frozen ``data/card_overrides.json`` boundary."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import re
from pathlib import Path
from typing import Any, Iterable

from .serialization import canonical_json_text


LEGACY_ARTIFACT = "data/card_overrides.json"
REFERENCE_PATTERN = re.compile(
    r"card_overrides\.json|\bCARD_OVERRIDES(?:_JSON)?\b|"
    r"\b(?:load|apply)_legacy_overrides\b|\bload_card_overrides\b|"
    r"\bapply_overrides\b"
)

# Every production-code reference must be classified here.  A new direct or
# indirect dependency therefore fails the checked inventory until it receives
# an explicit migration disposition.
REFERENCE_POLICY: dict[str, tuple[str, str]] = {
    "code/data_paths.py": ("path_declaration", "declares the frozen legacy path"),
    "code/tools/card_data/legacy_overrides.py": (
        "compatibility_reader",
        "the only direct reader; intentionally exposes no writer",
    ),
    "code/tools/ingest_cards.py": (
        "compatibility_consumer",
        "legacy command delegates read/apply behavior to the compatibility adapter",
    ),
    "code/tools/card_data_migration_baseline.py": (
        "migration_report_consumer",
        "counts frozen entries through the compatibility adapter",
    ),
    "code/tools/card_data/migrate_legacy_overrides.py": (
        "migration_generator",
        "converts every frozen field through the compatibility adapter",
    ),
    "code/tools/audit_digivolve/reconcile_traits.py": (
        "migrated_writer",
        "writes reviewed card_corrections.json entries, never legacy overrides",
    ),
    "code/tools/card_data/legacy_override_inventory.py": (
        "inventory_guard",
        "owns this reference inventory and direct-write detector",
    ),
}


def _source_files(repo_root: Path) -> Iterable[Path]:
    for path in sorted((repo_root / "code").rglob("*.py")):
        relative = path.relative_to(repo_root)
        if "__pycache__" in relative.parts or relative.parts[:2] == ("code", "tests"):
            continue
        yield path


def _matched_lines(text: str) -> tuple[int, ...]:
    return tuple(
        line_number
        for line_number, line in enumerate(text.splitlines(), start=1)
        if REFERENCE_PATTERN.search(line)
    )


def _expr_references_legacy(node: ast.AST | None, tainted: set[str]) -> bool:
    if node is None:
        return False
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return "card_overrides.json" in node.value
    if isinstance(node, ast.Name):
        return node.id in tainted or node.id in {"CARD_OVERRIDES", "CARD_OVERRIDES_JSON"}
    if isinstance(node, ast.Attribute) and node.attr in {
        "CARD_OVERRIDES",
        "CARD_OVERRIDES_JSON",
    }:
        return True
    return any(_expr_references_legacy(child, tainted) for child in ast.iter_child_nodes(node))


def _string_value(node: ast.AST | None) -> str | None:
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    return None


def _write_mode(call: ast.Call, *, positional_index: int) -> bool:
    mode_node = call.args[positional_index] if len(call.args) > positional_index else None
    for keyword in call.keywords:
        if keyword.arg == "mode":
            mode_node = keyword.value
    mode = _string_value(mode_node) or "r"
    return any(flag in mode for flag in "wax+")


def find_direct_legacy_writes(repo_root: Path) -> tuple[dict[str, Any], ...]:
    """Find statically evident mutations of the frozen legacy path."""

    findings: list[dict[str, Any]] = []
    mutating_methods = {"write_text", "write_bytes", "touch", "unlink", "rename", "replace"}
    for path in _source_files(repo_root):
        relative = path.relative_to(repo_root).as_posix()
        text = path.read_text(encoding="utf-8")
        if not REFERENCE_PATTERN.search(text):
            continue
        tree = ast.parse(text, filename=relative)
        tainted: set[str] = {"CARD_OVERRIDES", "CARD_OVERRIDES_JSON"}
        for node in ast.walk(tree):
            if isinstance(node, ast.ImportFrom):
                for alias in node.names:
                    if alias.name in {"CARD_OVERRIDES", "CARD_OVERRIDES_JSON"}:
                        tainted.add(alias.asname or alias.name)

        changed = True
        while changed:
            changed = False
            for node in ast.walk(tree):
                if isinstance(node, (ast.Assign, ast.AnnAssign)):
                    value = node.value
                    targets = node.targets if isinstance(node, ast.Assign) else [node.target]
                    if not _expr_references_legacy(value, tainted):
                        continue
                    for target in targets:
                        if isinstance(target, ast.Name) and target.id not in tainted:
                            tainted.add(target.id)
                            changed = True

        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            is_write = False
            operation = ""
            if isinstance(node.func, ast.Name) and node.func.id == "open":
                if node.args and _expr_references_legacy(node.args[0], tainted):
                    is_write = _write_mode(node, positional_index=1)
                    operation = "open"
            elif isinstance(node.func, ast.Attribute):
                method = node.func.attr
                owner = node.func.value
                if method == "open" and _expr_references_legacy(owner, tainted):
                    is_write = _write_mode(node, positional_index=0)
                    operation = "Path.open"
                elif method in mutating_methods and _expr_references_legacy(owner, tainted):
                    is_write = True
                    operation = method
                elif method in {"replace", "rename"} and any(
                    _expr_references_legacy(argument, tainted) for argument in node.args
                ):
                    is_write = True
                    operation = method
            if is_write:
                findings.append(
                    {"path": relative, "line": node.lineno, "operation": operation}
                )
    return tuple(sorted(findings, key=lambda item: (item["path"], item["line"])))


def build_inventory(repo_root: Path) -> dict[str, Any]:
    references: list[dict[str, Any]] = []
    discovered: set[str] = set()
    for path in _source_files(repo_root):
        relative = path.relative_to(repo_root).as_posix()
        text = path.read_text(encoding="utf-8")
        lines = _matched_lines(text)
        if not lines:
            continue
        discovered.add(relative)
        policy = REFERENCE_POLICY.get(relative)
        if policy is None:
            raise ValueError(f"unclassified legacy override reference: {relative}")
        role, disposition = policy
        references.append(
            {
                "path": relative,
                "role": role,
                "disposition": disposition,
                "reference_lines": list(lines),
                "source_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
            }
        )

    stale_policy = set(REFERENCE_POLICY) - discovered
    if stale_policy:
        raise ValueError(
            "legacy override inventory contains stale reference policies: "
            + ", ".join(sorted(stale_policy))
        )
    writes = find_direct_legacy_writes(repo_root)
    return {
        "schema_version": 1,
        "artifact": LEGACY_ARTIFACT,
        "policy": "compatibility_read_only_until_entry_migration",
        "direct_reader": "code/tools/card_data/legacy_overrides.py",
        "direct_writers": list(writes),
        "indirect_compatibility_consumers": [
            "code/tools/ingest_cards.py",
            "code/tools/card_data_migration_baseline.py",
            "code/tools/card_data/migrate_legacy_overrides.py",
        ],
        "migrated_writers": [
            {
                "path": "code/tools/audit_digivolve/reconcile_traits.py",
                "replacement": "data/card_corrections.json",
                "review_requirements": [
                    "evidence_observation_ids",
                    "reviewer",
                    "reviewed_at",
                    "reason",
                ],
            }
        ],
        "references": references,
    }


def main(argv: list[str] | None = None) -> int:
    default_root = Path(__file__).resolve().parents[3]
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=default_root)
    parser.add_argument(
        "--output",
        type=Path,
        default=default_root / "qa/qa-reports/card_overrides_inventory.json",
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    payload = canonical_json_text(build_inventory(args.repo_root.resolve()))
    if args.check:
        if not args.output.exists() or args.output.read_text(encoding="utf-8") != payload:
            return 1
        return 0
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(payload, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
