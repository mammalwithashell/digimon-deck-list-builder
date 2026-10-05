#!/usr/bin/env python3
"""One-time migration: exam clause ids after "Security Effect" moved to the security zone.

`clause_coverage.card_sources` mapped official text sections to clause zones by
label, but listed the security box as "Security" -- a label the official DB never
prints; it says "Security Effect". Every official security clause was therefore
zoned "effect" and took an effect index (`ST1-15#effect#1`), so its id moved
whenever the effect box's clause count changed. With the label mapped, the same
clause is `ST1-15#security#0`. Ids are positional and keyed into the exam's
verdicts and scenarios, so this script moves those keys to match:

  1. qa/qa-reports/exam-verdicts/<CARD>.json -- a verdict labelled "Security Effect"
     moves to its card's security-zone clause when its id is that clause's id under
     the old zoning, or (for verdicts an earlier renumbering already orphaned) when
     its text_sha256 equals that clause's text. Only the key and `clause_id` change;
     the verdict, hash, scenario_path and timestamps are kept, so a verdict whose
     hash was already stale stays invalidated.
  2. qa/dcgo-exams/**.yaml -- the `clause:` header (and the same id in the file's own
     comments) of the scenario that recorded each moved verdict. File names are kept,
     as the 2026-08-23 denominator correction kept them: `clause:` is the identity.
     A scenario whose header names an old security id but which no moved verdict
     vouches for is reported, not changed -- its line may test something else.
  3. qa/exam-clause-text.json -- the moved ids and zones, and denominator.by_zone.
  4. qa/dcgo-exams/**/NOTES-*.md -- the moved verdicts' clause ids. Job ids,
     recording names and file names contain no '#' and are untouched.

Not touched: qa/qa-reports/exam-log.jsonl (append-only history) and the dated
reports, which describe the ids as they were.

Usage (repo root, after the card_sources.py change):
    python code/tools/archive/migrate_security_effect_zone.py           # dry run
    python code/tools/archive/migrate_security_effect_zone.py --apply   # write
"""
import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

_CODE_DIR = Path(__file__).resolve().parents[2]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from tools.clause_coverage import card_sources as cs  # noqa: E402
from tools.clause_coverage.exam_binding import _parse_scenario_header  # noqa: E402

LABEL = "Security Effect"


def sha256_text(text: str) -> str:
    # Same normalisation as exam_binding.clause_text_sha256.
    return hashlib.sha256((text or "").replace("\r\n", "\n").replace("\r", "\n").encode("utf-8")).hexdigest()


def read_json(path: Path):
    """(data, writer) where writer(data) reproduces the file's exact serialisation."""
    raw = path.read_bytes()
    data = json.loads(raw.decode("utf-8"))
    eol = "\r\n" if b"\r\n" in raw else "\n"
    for ensure_ascii in (False, True):
        body = json.dumps(data, indent=2, ensure_ascii=ensure_ascii).replace("\n", eol)
        for tail in (eol, ""):
            if (body + tail).encode("utf-8") == raw:
                def write(new, ea=ensure_ascii, t=tail):
                    out = json.dumps(new, indent=2, ensure_ascii=ea).replace("\n", eol) + t
                    path.write_bytes(out.encode("utf-8"))
                return data, write
    raise SystemExit(f"{path}: serialisation is not json.dumps(indent=2); refusing to rewrite it")


def id_pattern(clause_id: str) -> str:
    # Whole ids only: not the tail of a longer card id, nor `#effect#1` inside `#effect#10`.
    return r"(?<![A-Za-z0-9-])" + re.escape(clause_id) + r"(?!\d)"


def replace_ids(text: str, mapping: dict[str, str]) -> str:
    for old, new in mapping.items():
        text = re.sub(id_pattern(old), new, text)
    return text


def extract(card_ids, data_dir: Path, *, security_effect_is_security: bool) -> dict:
    cards = cs.load_cards_index(data_dir / "cards.json")
    overrides = cs.load_overrides_index(data_dir / "card_overrides.json")
    official = cs.load_official_index(data_dir / "card_official.json")
    saved = dict(cs._LABEL_TO_ZONE)
    try:
        if security_effect_is_security:
            cs._LABEL_TO_ZONE["security effect"] = "security"
        else:
            cs._LABEL_TO_ZONE.pop("security effect", None)
        # Only bundled cards carry the label, and bundles never consult DCGO.
        return {cid: cs.extract_card_clauses(cid, cards_index=cards, overrides_index=overrides,
                                             official_index=official, dcgo_root=None)
                for cid in card_ids}
    finally:
        cs._LABEL_TO_ZONE.clear()
        cs._LABEL_TO_ZONE.update(saved)


def main(argv=None) -> int:
    root_default = Path(__file__).resolve().parents[3]
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--apply", action="store_true", help="write the changes (default: dry run)")
    ap.add_argument("--root", type=Path, default=root_default, help="repo root holding qa/")
    ap.add_argument("--data-dir", type=Path, default=None, help="card data dir (default: <root>/data)")
    args = ap.parse_args(argv)
    root, data_dir = args.root, args.data_dir or args.root / "data"

    if cs._LABEL_TO_ZONE.get("security effect") != "security":
        raise SystemExit("card_sources does not zone 'Security Effect' as security yet; apply that change first")

    verdict_dir = root / "qa" / "qa-reports" / "exam-verdicts"
    scenario_dir = root / "qa" / "dcgo-exams"
    book_path = root / "qa" / "exam-clause-text.json"
    verdict_files = sorted(verdict_dir.glob("*.json"))
    scenarios = {p: _parse_scenario_header(p.read_text(encoding="utf-8")) for p in sorted(scenario_dir.rglob("*.yaml"))}
    book, write_book = read_json(book_path)

    card_ids = sorted(
        {f.stem for f in verdict_files}
        | {c["card_id"] for c in book["clauses"]}
        | {h["clause"].split("#")[0] for h in scenarios.values() if h["clause"]}
    )
    old = extract(card_ids, data_dir, security_effect_is_security=False)
    new = extract(card_ids, data_dir, security_effect_is_security=True)
    id_map, old_by_id = {}, {}
    for cid in card_ids:
        assert len(old[cid]) == len(new[cid]), cid
        for a, b in zip(old[cid], new[cid]):
            assert (a.text, a.label) == (b.text, b.label), (a.id, b.id)
            id_map[a.id] = b.id
            old_by_id[a.id] = a

    # 1. verdicts ------------------------------------------------------------------
    moved: dict[str, str] = {}
    moved_scenarios: dict[Path, tuple[str, str]] = {}
    report: list[str] = []
    for f in verdict_files:
        store, write_store = read_json(f)
        clauses = store["clauses"]
        renames = {}
        for vid, entry in clauses.items():
            if entry.get("label") != LABEL or vid.split("#")[1:2] == ["security"]:
                continue
            current = old_by_id.get(vid)
            if current is not None and current.label == LABEL:
                target = id_map[vid]
            else:
                # Orphaned, or re-pointed at another clause, by an earlier renumbering:
                # follow the text hash instead of the position.
                hits = [c.id for c in new[f.stem] if c.zone == "security" and sha256_text(c.text) == entry.get("text_sha256")]
                target = hits[0] if len(hits) == 1 else None
            if target is None:
                report.append(f"left alone: verdict {vid} (labelled {LABEL!r}, matches no security clause)")
                continue
            assert target.split("#")[1] == "security", (vid, target)
            if target in clauses or target in renames.values():
                report.append(f"left alone: verdict {vid} -> {target} would overwrite an existing verdict")
                continue
            renames[vid] = target
        if not renames:
            continue
        rebuilt = {}
        for vid, entry in clauses.items():
            key = renames.get(vid, vid)
            rebuilt[key] = dict(entry, clause_id=key) if vid in renames else entry
        if list(clauses) == sorted(clauses):  # the Rust VerdictStore writes a BTreeMap
            rebuilt = dict(sorted(rebuilt.items()))
        store["clauses"] = rebuilt
        for vid, target in renames.items():
            moved[vid] = target
            sp = clauses[vid].get("scenario_path")
            if sp:
                moved_scenarios[root / sp] = (vid, target)
            print(f"verdict   {f.relative_to(root)}: {vid} -> {target} ({clauses[vid]['verdict']})")
        if args.apply:
            write_store(store)

    # 2. scenario headers ------------------------------------------------------------
    retargeted = 0
    for path, (vid, target) in sorted(moved_scenarios.items()):
        header = scenarios.get(path)
        if header is None or header["clause"] != vid:
            report.append(f"left alone: {path.relative_to(root)} (verdict {vid} names it, but its clause: is {header and header['clause']})")
            continue
        print(f"scenario  {path.relative_to(root)}: clause {vid} -> {target}")
        retargeted += 1
        if args.apply:
            raw = path.read_bytes().decode("utf-8")
            path.write_bytes(replace_ids(raw, {vid: target}).encode("utf-8"))
    old_security = {a for a, b in id_map.items() if a != b}
    for path, header in scenarios.items():
        if header["clause"] in old_security and path not in moved_scenarios:
            report.append(
                f"left alone: {path.relative_to(root)} names {header['clause']}, the security clause's OLD id, "
                "but no moved verdict vouches that it tests that clause; after the fix it binds to nothing"
            )

    # 3. clause-text book -------------------------------------------------------------
    book_moves = 0
    for clause in book["clauses"]:
        target = id_map.get(clause["id"])
        if target and target != clause["id"]:
            new_clause = next(c for c in new[clause["card_id"]] if c.id == target)
            assert clause["text"] == new_clause.text, clause["id"]
            print(f"book      {clause['id']} -> {target}")
            clause["id"], clause["zone"] = target, new_clause.zone
            book_moves += 1
    if book_moves:
        counts = Counter(c["zone"] for c in book["clauses"])
        by_zone = book["denominator"]["by_zone"]
        book["denominator"]["by_zone"] = {z: counts[z] for z in [*by_zone, *(z for z in counts if z not in by_zone)] if counts[z]}
        book["source"] += f"; {book_moves} 'Security Effect' ids moved to the security zone (2026-10-03)"
        if args.apply:
            write_book(book)

    # 4. NOTES ------------------------------------------------------------------------
    for path in sorted(scenario_dir.rglob("NOTES-*.md")):
        raw = path.read_bytes().decode("utf-8")
        out = replace_ids(raw, moved)
        if out != raw:
            n = sum(len(re.findall(id_pattern(o), raw)) for o in moved)
            print(f"notes     {path.relative_to(root)}: {n} id(s)")
            if args.apply:
                path.write_bytes(out.encode("utf-8"))

    print(f"\n{len(moved)} verdicts, {retargeted} scenario headers, {book_moves} clause-text-book ids"
          + ("" if args.apply else "  (dry run: nothing written)"))
    for line in report:
        print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
