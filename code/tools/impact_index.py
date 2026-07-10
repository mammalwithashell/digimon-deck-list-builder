#!/usr/bin/env python3
"""Build the DSL impact index used by scoped engine verification.

The index maps DSL step verbs, predicate keys, and timing strings to the card
IDs whose YAML uses them. It intentionally scans parsed YAML, not comments or
raw text, and it normalizes known serde aliases to their canonical DSL keys.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml

REPO = Path(__file__).resolve().parents[2]
DEFAULT_CARDS_DIR = REPO / "code" / "digimon-engine" / "cards"
DEFAULT_OUT = REPO / "qa" / "impact" / "dsl_impact_index.json"
STEP_RS = REPO / "code" / "digimon-dsl" / "src" / "step.rs"
PREDICATE_RS = REPO / "code" / "digimon-dsl" / "src" / "predicate.rs"

SCHEMA_VERSION = 1

STEP_ALIASES = {
    "gain_memory_fn": "gain_memory",
    "lose_memory_fn": "lose_memory",
}

PREDICATE_ALIASES = {
    "trait": "trait_has",
    "subject_trait": "trait_has",
    "of": "owner",
    "binding_is_present": "binding_present",
    "binding_is_none": "binding_absent",
    "any_returned_card": "effect_returned_any_card",
    "all": "all_of",
}

STEP_LIST_KEYS = {
    "body",
    "else",
    "extra_cost",
    "on_burst_turn_end",
    "on_no_match",
    "on_select",
    "pay_cost",
    "process",
    "then",
}

PREDICATE_CONTAINER_KEYS = {
    "active_when",
    "applies_to",
    "card_filter",
    "condition",
    "cost_target",
    "filter",
    "from",
    "hand",
    "host_filter",
    "left_filter",
    "material",
    "material_carrier_filter",
    "over",
    "overclock_cost_filter",
    "predicate",
    "result_filter",
    "right_filter",
    "sources",
    "target",
    "targets",
    "trash",
    "use_requirement",
    "when_any_ally_digivolves_into",
    "when_any_ally_played",
    "while_condition",
}


@dataclass
class CardUsage:
    card_id: str
    path: str
    verbs: set[str] = field(default_factory=set)
    predicates: set[str] = field(default_factory=set)
    timings: set[str] = field(default_factory=set)


def load_step_keys(step_rs: Path = STEP_RS) -> set[str]:
    text = step_rs.read_text(encoding="utf-8")
    keys = set(re.findall(r'kv!\(\s*s\s*,\s*"([a-z0-9_]+)"', text))
    keys.update(STEP_ALIASES.values())
    return keys


def load_predicate_keys(predicate_rs: Path = PREDICATE_RS) -> set[str]:
    text = predicate_rs.read_text(encoding="utf-8")
    try:
        body = text.split("pub struct PredicateSpec", 1)[1].split(
            "\n}\n\nimpl PredicateSpec", 1
        )[0]
    except IndexError as exc:
        raise RuntimeError(f"could not locate PredicateSpec in {predicate_rs}") from exc
    keys = set(re.findall(r"\bpub\s+([a-zA-Z0-9_]+)\s*:", body))
    keys.discard("extra")
    keys.update(PREDICATE_ALIASES.values())
    return keys


def build_impact_index(
    cards_dir: Path = DEFAULT_CARDS_DIR,
    *,
    step_keys: set[str] | None = None,
    predicate_keys: set[str] | None = None,
) -> dict[str, Any]:
    cards_dir = Path(cards_dir)
    step_keys = step_keys or load_step_keys()
    predicate_keys = predicate_keys or load_predicate_keys()

    cards: dict[str, CardUsage] = {}
    for path in sorted(cards_dir.rglob("*.yaml")):
        if path.name.startswith("."):
            continue
        raw = path.read_text(encoding="utf-8")
        data = yaml.safe_load(raw)
        if not isinstance(data, dict):
            continue
        card_id = str(data.get("card") or path.stem)
        usage = CardUsage(card_id=card_id, path=path.relative_to(cards_dir).as_posix())
        scan_general(data, usage, step_keys, predicate_keys)
        cards[card_id] = usage

    return materialize_index(cards, cards_dir)


def scan_general(
    node: Any,
    usage: CardUsage,
    step_keys: set[str],
    predicate_keys: set[str],
    *,
    step_context: bool = False,
) -> None:
    if isinstance(node, list):
        for item in node:
            scan_general(
                item,
                usage,
                step_keys,
                predicate_keys,
                step_context=step_context,
            )
        return
    if not isinstance(node, dict):
        return

    if step_context and len(node) == 1:
        raw_key = next(iter(node))
        verb = STEP_ALIASES.get(str(raw_key), str(raw_key))
        if verb in step_keys:
            usage.verbs.add(verb)
            scan_general(
                node[raw_key],
                usage,
                step_keys,
                predicate_keys,
                step_context=isinstance(node[raw_key], list),
            )
            return

    for raw_key, value in node.items():
        key = str(raw_key)
        if key == "when":
            collect_timings(value, usage)
        if key in STEP_LIST_KEYS:
            scan_general(value, usage, step_keys, predicate_keys, step_context=True)
        elif key in PREDICATE_CONTAINER_KEYS:
            scan_predicate(value, usage, predicate_keys)
        else:
            scan_general(value, usage, step_keys, predicate_keys)


def scan_predicate(node: Any, usage: CardUsage, predicate_keys: set[str]) -> None:
    if isinstance(node, list):
        for item in node:
            scan_predicate(item, usage, predicate_keys)
        return
    if not isinstance(node, dict):
        return

    for raw_key, value in node.items():
        key = PREDICATE_ALIASES.get(str(raw_key), str(raw_key))
        if key in predicate_keys:
            usage.predicates.add(key)
        scan_predicate(value, usage, predicate_keys)


def collect_timings(value: Any, usage: CardUsage) -> None:
    if isinstance(value, str):
        usage.timings.add(value)
    elif isinstance(value, list):
        for item in value:
            if isinstance(item, str):
                usage.timings.add(item)


def materialize_index(cards: dict[str, CardUsage], cards_dir: Path) -> dict[str, Any]:
    card_payload = {
        card_id: {
            "path": usage.path,
            "verbs": sorted(usage.verbs),
            "predicates": sorted(usage.predicates),
            "timings": sorted(usage.timings),
        }
        for card_id, usage in sorted(cards.items())
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "cards_root": cards_dir.as_posix(),
        "cards": card_payload,
        "verbs": invert(cards, "verbs"),
        "predicates": invert(cards, "predicates"),
        "timings": invert(cards, "timings"),
    }


def invert(cards: dict[str, CardUsage], field_name: str) -> dict[str, list[str]]:
    out: dict[str, set[str]] = {}
    for card_id, usage in cards.items():
        for key in getattr(usage, field_name):
            out.setdefault(key, set()).add(card_id)
    return {key: sorted(values) for key, values in sorted(out.items())}


def write_index(index: dict[str, Any], out: Path) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(index, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")


def cmd_build(args: argparse.Namespace) -> int:
    write_index(build_impact_index(args.cards_dir), args.out)
    print(f"wrote DSL impact index to {args.out}")
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    expected = build_impact_index(args.cards_dir)
    expected_text = json.dumps(expected, indent=2, sort_keys=True) + "\n"
    try:
        current = args.out.read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"missing impact index: {args.out}", file=sys.stderr)
        return 1
    if current != expected_text:
        print(
            f"impact index is stale: {args.out}\n"
            "Run `python code/tools/impact_index.py --out qa/impact/dsl_impact_index.json`.",
            file=sys.stderr,
        )
        return 1
    print(f"impact index is current: {args.out}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cards-dir", type=Path, default=DEFAULT_CARDS_DIR)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--check", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.check:
        return cmd_check(args)
    return cmd_build(args)


if __name__ == "__main__":
    raise SystemExit(main())
