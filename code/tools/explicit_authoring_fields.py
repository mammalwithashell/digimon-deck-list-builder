"""Insert explicit authoring choices without reserializing card YAML.

Snapshot compilation first with verify_authoring_fields, then run this tool
and compare against that snapshot with the same binary's verify command.
"""
from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path
import re

import yaml
from yaml.nodes import MappingNode, SequenceNode

ROOT = Path(__file__).resolve().parents[2]


def optional_steps():
    source = (ROOT / "code/digimon-dsl/src/authoring_fields.rs").read_text()
    body = source.split("pub const OPTIONAL_STEPS:", 1)[1].split("];", 1)[0]
    return set(re.findall(r'"([a-z_]+)"', body))


def fields(node):
    return {key.value: value for key, value in node.value} if isinstance(node, MappingNode) else {}


def has_level(node):
    mapping = fields(node)
    return any(key == "level" or key.startswith("level_") for key in mapping) or any(
        has_level(child) for key in ("all_of", "any_of")
        for child in getattr(mapping.get(key), "value", [])
    )


def explicit_color(node):
    mapping = fields(node)
    if "color_is" in mapping or getattr(mapping.get("any_color"), "value", None) == "true":
        return True
    all_of = getattr(mapping.get("all_of"), "value", [])
    any_of = getattr(mapping.get("any_of"), "value", [])
    return any(explicit_color(child) for child in all_of) or bool(any_of) and all(explicit_color(child) for child in any_of)


def annotate(raw):
    node = yaml.compose(raw)
    required = {}
    verbs = optional_steps()

    def add(mapping, key, value):
        if not isinstance(mapping, MappingNode):
            raise ValueError(f"{key} requires a mapping")
        if key not in fields(mapping):
            required[(mapping.start_mark.index, key)] = (mapping, key, value)

    def visit(node):
        if isinstance(node, MappingNode):
            for key, child in node.value:
                if key.value in verbs:
                    add(child, "optional", "false")
                if key.value == "reveal_search":
                    for bucket in getattr(fields(child).get("buckets"), "value", []):
                        add(bucket, "optional", "false")
                if key.value == "alt_paths":
                    for entry in getattr(child, "value", []):
                        entry = fields(entry)
                        if (getattr(entry.get("kind"), "value", None) == "digivolve"
                            and getattr(entry.get("direction"), "value", "from") == "from"
                            and has_level(entry.get("from")) and not explicit_color(entry.get("from"))):
                            add(entry["from"], "any_color", "true")
                visit(child)
        elif isinstance(node, SequenceNode):
            for child in node.value:
                visit(child)

    visit(node)
    edits = []
    counts = Counter()
    for node, key, value in required.values():
        counts[key] += 1
        if node.flow_style:
            start = node.value[0][0].start_mark.index if node.value else node.end_mark.index - 1
            edits.append((start, f"{key}: {value}" + (", " if node.value else "")))
        else:
            start = node.value[0][0].start_mark.index
            end = raw.find("\n", start)
            newline = "\r\n" if end > 0 and raw[end - 1] == "\r" else "\n"
            edits.append((start, f"{key}: {value}{newline}" + " " * node.value[0][0].start_mark.column))
    for index, text in sorted(edits, reverse=True):
        raw = raw[:index] + text + raw[index:]
    yaml.safe_load(raw)
    return raw, counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cards-dir", type=Path, default=ROOT / "code/digimon-engine/cards")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    total = Counter()
    changed = 0
    for path in sorted(args.cards_dir.rglob("*.yaml")):
        if any(part.startswith("_") or part == "test" for part in path.relative_to(args.cards_dir).parts[:-1]):
            continue
        original = path.read_bytes().decode()
        updated, counts = annotate(original)
        if updated != original:
            changed += 1
            total.update(counts)
            if args.write:
                path.write_bytes(updated.encode())
    print(f"explicit authoring fields: {dict(total)}; {changed} files {'written' if args.write else 'would change'}")


if __name__ == "__main__":
    main()
