#!/usr/bin/env python3
"""Compute a conservative behavior-test scope from changed paths."""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

CODE_DIR = Path(__file__).resolve().parents[1]
if str(CODE_DIR) not in sys.path:
    sys.path.insert(0, str(CODE_DIR))

from tools.impact_index import DEFAULT_OUT as DEFAULT_DSL_INDEX
from tools.impact_lowering_map import DEFAULT_MAP as DEFAULT_LOWERING_MAP
from tools.impact_lowering_map import load_lowering_map

ENGINE_SRC_PREFIXES = (
    "code/digimon-engine/src/",
    "code/digimon-dsl/src/",
)
CARD_YAML_PREFIX = "code/digimon-engine/cards/"


@dataclass(frozen=True)
class ImpactScopeResult:
    full_suite_required: bool
    cards: list[str]
    verbs: list[str]
    side_binaries: list[str]
    reasons: list[str]

    @property
    def cards_behavioral_filter(self) -> str:
        return "|".join(self.cards)

    def to_json(self) -> dict[str, Any]:
        return {
            "full_suite_required": self.full_suite_required,
            "cards_behavioral_filter": self.cards_behavioral_filter,
            "cards": self.cards,
            "verbs": self.verbs,
            "side_binaries": self.side_binaries,
            "reasons": self.reasons,
        }


def load_dsl_index(path: Path = DEFAULT_DSL_INDEX) -> dict[str, Any]:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def scope_for_changed_paths(
    paths: list[str],
    *,
    dsl_index: dict[str, Any],
    lowering_map: dict[str, list[str]],
) -> ImpactScopeResult:
    cards: set[str] = set()
    verbs: set[str] = set()
    side_binaries: set[str] = set()
    reasons: list[str] = []
    full_suite_required = False
    card_by_path = card_path_index(dsl_index)

    for raw in paths:
        path = normalize_path(raw)
        if not path:
            continue
        if path in lowering_map:
            mapped_verbs = lowering_map[path]
            verbs.update(mapped_verbs)
            for verb in mapped_verbs:
                cards.update(dsl_index.get("verbs", {}).get(verb, []))
            side_binaries.add("dsl")
            continue
        if path.startswith(CARD_YAML_PREFIX) and path.endswith((".yaml", ".yml")):
            card_id = card_by_path.get(path) or Path(path).stem
            cards.add(card_id)
            side_binaries.add("dsl")
            continue
        if path.startswith("code/digimon-engine/tests/"):
            side_binaries.add(engine_test_binary_for_path(path))
            continue
        if path.startswith(ENGINE_SRC_PREFIXES):
            full_suite_required = True
            reasons.append(f"{path} is an unmapped engine/core file")

    return ImpactScopeResult(
        full_suite_required=full_suite_required,
        cards=[] if full_suite_required else sorted(cards),
        verbs=sorted(verbs),
        side_binaries=sorted(side_binaries),
        reasons=reasons,
    )


def card_path_index(dsl_index: dict[str, Any]) -> dict[str, str]:
    out: dict[str, str] = {}
    for card_id, payload in dsl_index.get("cards", {}).items():
        rel = payload.get("path")
        if isinstance(rel, str):
            out[f"{CARD_YAML_PREFIX}{rel}"] = card_id
    return out


def normalize_path(path: str) -> str:
    return path.strip().replace("\\", "/").lstrip("./")


def engine_test_binary_for_path(path: str) -> str:
    rel = path.removeprefix("code/digimon-engine/tests/")
    first = rel.split("/", 1)[0]
    return first.removesuffix(".rs")


def changed_paths_from_git(base: str | None, head: str | None) -> list[str]:
    cmd = ["git", "diff", "--name-only"]
    if base and head:
        cmd.extend([base, head])
    elif base:
        cmd.append(base)
    result = subprocess.run(cmd, check=True, capture_output=True, text=True)
    return [line for line in result.stdout.splitlines() if line.strip()]


def changed_paths_from_args(args: argparse.Namespace) -> list[str]:
    if args.path:
        return args.path
    if args.paths_file:
        return [
            line
            for line in args.paths_file.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
    return changed_paths_from_git(args.base, args.head)


def render_text(result: ImpactScopeResult) -> str:
    lines = [
        f"full_suite_required: {str(result.full_suite_required).lower()}",
        f"cards_behavioral_filter: {result.cards_behavioral_filter}",
        f"cards: {', '.join(result.cards)}",
        f"verbs: {', '.join(result.verbs)}",
        f"side_binaries: {', '.join(result.side_binaries)}",
    ]
    if result.reasons:
        lines.append("reasons:")
        lines.extend(f"- {reason}" for reason in result.reasons)
    return "\n".join(lines)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dsl-index", type=Path, default=DEFAULT_DSL_INDEX)
    parser.add_argument("--lowering-map", type=Path, default=DEFAULT_LOWERING_MAP)
    parser.add_argument("--path", action="append", help="changed path; may be repeated")
    parser.add_argument("--paths-file", type=Path)
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--json", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    result = scope_for_changed_paths(
        changed_paths_from_args(args),
        dsl_index=load_dsl_index(args.dsl_index),
        lowering_map=load_lowering_map(args.lowering_map),
    )
    if args.json:
        print(json.dumps(result.to_json(), indent=2, sort_keys=True))
    else:
        print(render_text(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
