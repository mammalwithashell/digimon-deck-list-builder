#!/usr/bin/env python3
"""Validate the engine lowering-file to DSL-verb impact map."""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

CODE_DIR = Path(__file__).resolve().parents[1]
if str(CODE_DIR) not in sys.path:
    sys.path.insert(0, str(CODE_DIR))

from tools.impact_index import REPO, load_step_keys

DEFAULT_MAP = REPO / "qa" / "impact" / "lowering_verb_map.json"


@dataclass(frozen=True)
class LoweringMapCheckResult:
    missing_verbs: list[str]
    unknown_verbs: list[str]
    missing_files: list[str]
    duplicate_file_verbs: list[str]

    @property
    def ok(self) -> bool:
        return not (
            self.missing_verbs
            or self.unknown_verbs
            or self.missing_files
            or self.duplicate_file_verbs
        )


def load_lowering_map(path: Path = DEFAULT_MAP) -> dict[str, list[str]]:
    raw = json.loads(Path(path).read_text(encoding="utf-8"))
    if raw.get("schema_version") != 1:
        raise ValueError(f"{path} has unsupported schema_version")
    files = raw.get("files")
    if not isinstance(files, dict):
        raise ValueError(f"{path} missing files object")

    out: dict[str, list[str]] = {}
    for file_name, verbs in files.items():
        if not isinstance(file_name, str) or not isinstance(verbs, list):
            raise ValueError(f"{path} files entries must be string -> list[str]")
        if not all(isinstance(verb, str) for verb in verbs):
            raise ValueError(f"{path} verb lists must contain only strings")
        out[file_name] = verbs
    return out


def check_lowering_map(
    lowering_map: dict[str, list[str]],
    *,
    authoritative_verbs: set[str] | None = None,
    repo: Path = REPO,
) -> LoweringMapCheckResult:
    authoritative_verbs = authoritative_verbs or load_step_keys()
    mapped: set[str] = set()
    duplicate_file_verbs: list[str] = []
    missing_files: list[str] = []

    for file_name, verbs in sorted(lowering_map.items()):
        if not (repo / file_name).exists():
            missing_files.append(file_name)
        seen_in_file: set[str] = set()
        for verb in verbs:
            if verb in seen_in_file:
                duplicate_file_verbs.append(f"{file_name}:{verb}")
            seen_in_file.add(verb)
            mapped.add(verb)

    return LoweringMapCheckResult(
        missing_verbs=sorted(authoritative_verbs - mapped),
        unknown_verbs=sorted(mapped - authoritative_verbs),
        missing_files=sorted(missing_files),
        duplicate_file_verbs=sorted(duplicate_file_verbs),
    )


def result_to_lines(result: LoweringMapCheckResult) -> list[str]:
    lines: list[str] = []
    if result.missing_verbs:
        lines.append("missing verbs: " + ", ".join(result.missing_verbs))
    if result.unknown_verbs:
        lines.append("unknown verbs: " + ", ".join(result.unknown_verbs))
    if result.missing_files:
        lines.append("missing files: " + ", ".join(result.missing_files))
    if result.duplicate_file_verbs:
        lines.append("duplicate file verb entries: " + ", ".join(result.duplicate_file_verbs))
    return lines


def cmd_check(args: argparse.Namespace) -> int:
    result = check_lowering_map(load_lowering_map(args.map), repo=args.repo)
    if result.ok:
        print(f"lowering impact map is current: {args.map}")
        return 0
    for line in result_to_lines(result):
        print(line, file=sys.stderr)
    return 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--map", type=Path, default=DEFAULT_MAP)
    parser.add_argument("--repo", type=Path, default=REPO)
    parser.add_argument("--check", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.check:
        return cmd_check(args)
    parser.error("only --check is currently supported")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
