"""Conservatively annotate YAML clauses from official printed sections.

The clause_coverage package has source extraction, activation-to-denominator
matching, and specialized Link/Assembly bindings, but no general YAML-clause
join. Reuse its splitter here, retaining original spans rather than its stripped
bodies. Ambiguous joins are deliberately left for manual authoring.

Run from the repository root:
  python -m tools.backfill_clause_text --output-dir ~/clause-text-backfill --write
Every edit is staged and verified by the DSL loader/compiler before any write.
"""

from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import dataclass
import json
from pathlib import Path
import re
import subprocess

import yaml
from yaml.nodes import MappingNode, SequenceNode

from tools.clause_coverage.text_split import split_clauses


ROOT = Path(__file__).resolve().parents[2]
TIMINGS = {
    "on play": "on_play", "when digivolving": "when_digivolving",
    "when attacking": "when_attacking", "on deletion": "on_deletion",
    "main": "main", "security": "on_security", "counter": "counter",
    "start of your turn": "start_of_your_turn",
    "start of opponent's turn": "start_of_opponents_turn",
    "start of your main phase": "start_of_your_main_phase",
    "start of opponent's main phase": "start_of_opponents_main_phase",
    "end of your turn": "end_of_your_turn",
    "end of opponent's turn": "end_of_opponents_turn",
    "end of all turns": "end_of_all_turns", "end of attack": "end_of_attack",
}
GATES = {"your turn": "your_turn", "opponent's turn": "opponents_turn", "all turns": "all_turns"}
MARKER = re.compile(r"\[([^\[\]]+)\]|[<＜]([^<>＜＞]+)[>＞]")
COLOR_REQUIREMENT = re.compile(r"\bwhile\b[^.]*\bignore\b[^.]*\bcolou?r requirements\.", re.IGNORECASE)


def normalized(text):
    return " ".join(text.replace("’", "'").replace("ʼ", "'").split()).casefold()


@dataclass
class Span:
    text: str
    zone: str
    face: str
    tags: list[str]
    keyword: str | None


def printed_spans(sections):
    result = []
    for section in sections:
        label, raw = section["label"].casefold(), section["text"]
        if label not in {"effect", "inherited effect", "security effect", "dual effect"}:
            continue
        zone = {"inherited effect": "inherited", "security effect": "security"}.get(label, "effect")
        face = "option" if label == "dual effect" else "digimon"
        spans = split_clauses(raw)
        located, cursor = [], 0
        for span in spans:
            name = span.timings[0] if span.timings else span.keyword
            if name:
                marker = next((m for m in MARKER.finditer(raw, cursor)
                               if normalized((m[1] or m[2]).split("(", 1)[0]) == normalized(name)), None)
                if marker is None:
                    located = []
                    break
                start = marker.start()
                cursor = marker.end()
                for tag in span.timings[1:]:
                    following = MARKER.match(raw, cursor + len(raw[cursor:]) - len(raw[cursor:].lstrip()))
                    if following is None or normalized(following[1] or following[2]) != normalized(tag):
                        located = []
                        break
                    cursor = following.end()
                else:
                    if span.text:
                        body = re.search(r"\s+".join(re.escape(w) for w in span.text.split()), raw[cursor:])
                        if body is None:
                            located = []
                            break
                        cursor += body.end()
                    located.append((start, span))
                    continue
                break
            else:
                start = raw.find(span.text, cursor)
                if start < 0:
                    located = []
                    break
                cursor = start + len(span.text)
            located.append((start, span))
        groups = []
        hand_start = None
        for i, (start, span) in enumerate(located):
            end = located[i + 1][0] if i + 1 < len(located) else len(raw)
            if (normalized(raw[start:end]) in {"[hand]", "{hand}"}
                    and i + 1 < len(located) and "Main" in located[i + 1][1].timings):
                hand_start = start
                continue
            if groups and span.kind == "keyword" and re.search(r"[・•]\s*$", raw[:start]):
                continue
            groups.append((hand_start if hand_start is not None else start, span))
            hand_start = None
        for i, (start, span) in enumerate(groups):
            end = groups[i + 1][0] if i + 1 < len(groups) else len(raw)
            text = raw[start:end].split("(Rule)", 1)[0].strip()
            requirement = COLOR_REQUIREMENT.search(text)
            if not span.timings and not span.keyword and requirement:
                text = requirement.group()
            span_zone = "security" if "security" in [normalized(t) for t in span.timings] else zone
            if text and (span.text or span.kind == "keyword"):
                result.append(Span(text, span_zone, face, [normalized(t) for t in span.timings], span.keyword))
    return result


def gate(predicate, name):
    return isinstance(predicate, dict) and (predicate.get(name) is True
        or any(gate(p, name) for p in predicate.get("all_of", [])))


def compatible(clause, face, span):
    if face != span.face or clause.get("scope") in {"both", "linked", "security"}:
        return False
    when = clause.get("when", [])
    when = {when} if isinstance(when, str) else set(when)
    zone = "security" if "on_security" in when else ("inherited" if clause.get("scope") == "inherited" else "effect")
    if zone != span.zone:
        return False
    if any(tag not in TIMINGS and tag not in GATES and tag not in {"once per turn", "delay"}
           for tag in span.tags):
        return False
    events = {TIMINGS[t] for t in span.tags if t in TIMINGS}
    if span.zone == "security" and not events:
        events = {"on_security"}
    if "delay" in span.tags:
        return clause.get("kind") == "delay" and clause.get("trigger") == "delayed" and events == {"main"}
    if clause.get("kind") == "delay":
        return False
    if "main_from_hand" in when:
        when = (when - {"main_from_hand"}) | {"main"}
    if events and events != when:
        return False
    if not events and COLOR_REQUIREMENT.search(span.text):
        return clause.get("kind") == "flood_gate" and clause.get("modifier") == "IgnoreColorRequirement"
    for tag in span.tags:
        if tag not in GATES:
            continue
        active = clause.get("active_when", {})
        if tag == "all turns":
            if gate(active, "your_turn") or gate(active, "opponents_turn"):
                return False
        elif not gate(active, GATES[tag]):
            return False
    if span.keyword:
        if clause.get("kind") != "grant_keyword":
            return False
        keyword = re.sub(r"[^a-z]", "", normalized(span.keyword))
        authored = re.sub(r"[^a-z]", "", str(clause.get("keyword", "")).casefold())
        return keyword == authored
    return bool(events or span.tags or not when)


def mapping(node):
    return {k.value: v for k, v in node.value} if isinstance(node, MappingNode) else {}


def clauses(document, node):
    roots = [(document, node, "digimon")]
    dual = document.get("dual", {})
    dual_node = mapping(mapping(node).get("dual"))
    roots.extend((dual[face], dual_node[face], face) for face in ("digimon", "option") if face in dual)
    result = []
    for obj, root, face in roots:
        effects = mapping(root).get("effects")
        if isinstance(effects, SequenceNode):
            result.extend(zip(obj["effects"], effects.value, [face] * len(effects.value)))
    return result


def annotate(raw, sections):
    document = yaml.safe_load(raw)
    entries = clauses(document, yaml.compose(raw))
    spans = printed_spans(sections)
    candidates = [[i for i, span in enumerate(spans) if compatible(clause, face, span)]
                  for clause, _, face in entries]
    usage = Counter(i for possible in candidates for i in possible)
    insertions = []
    for (clause, node, _), possible in zip(entries, candidates):
        if "text" in clause or len(possible) != 1 or usage[possible[0]] != 1:
            continue
        start = node.start_mark.index
        line_start = raw.rfind("\n", 0, start) + 1
        # Only block sequence mappings are safe for line-preserving insertion.
        if not re.fullmatch(r" *- +", raw[line_start:start]) or node.flow_style:
            continue
        encoded = json.dumps(spans[possible[0]].text, ensure_ascii=False)
        line_end = raw.find("\n", start)
        newline = "\r\n" if line_end > 0 and raw[line_end - 1] == "\r" else "\n"
        insertions.append((start, f"text: {encoded}{newline}{' ' * node.start_mark.column}"))
    updated = raw
    for start, insertion in reversed(insertions):
        updated = updated[:start] + insertion + updated[start:]
    return updated, len(insertions), len(entries)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--cards", nargs="+", help="Limit annotation to these card IDs")
    args = parser.parse_args()
    root, output = args.root.resolve(), args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=False)
    official = json.loads((root / "data/card_official.json").read_text())["cards"]
    edits, assigned, total = [], 0, 0
    for path in sorted((root / "code/digimon-engine/cards").glob("*/*.yaml")):
        if path.parent.name.startswith("_") or path.parent.name == "test":
            continue
        raw = path.read_bytes().decode("utf-8")
        card = yaml.safe_load(raw)["card"]
        if args.cards and card not in args.cards:
            continue
        updated, count, clauses_count = annotate(raw, official.get(card, {}).get("text_sections", []))
        assigned += count
        total += clauses_count
        if updated == raw:
            continue
        before, after = output / "before" / path.name, output / "after" / path.name
        before.parent.mkdir(exist_ok=True)
        after.parent.mkdir(exist_ok=True)
        before.write_bytes(raw.encode("utf-8"))
        after.write_bytes(updated.encode("utf-8"))
        edits.append({"path": str(path), "before": str(before), "after": str(after)})
    manifest = output / "edits.json"
    manifest.write_text(json.dumps(edits, indent=2) + "\n")
    subprocess.run(["cargo", "run", "--quiet", "-p", "digimon-dsl", "--bin", "verify_clause_text", "--", str(manifest)], cwd=root, check=True)
    if args.write:
        for edit in edits:
            path = Path(edit["path"])
            if path.read_bytes() != Path(edit["before"]).read_bytes():
                raise RuntimeError(f"concurrent edit: {path}")
        for edit in edits:
            Path(edit["path"]).write_bytes(Path(edit["after"]).read_bytes())
    print(f"backfill: {assigned} clauses assigned / {total} total; {len(edits)} files {'written' if args.write else 'staged'}")


if __name__ == "__main__":
    main()
