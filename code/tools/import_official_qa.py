#!/usr/bin/env python3
"""Fold Bandai's official card Q&A rulings into our offline mirror, with provenance.

Our mirror (data/card_official.json + data/card_bundles/<ID>.md, built by
build_card_bundles.py from the official *card list* page) keeps only the first ruling's
answer per card: no question, no Q-number, no date, and the HTML parse drops <Keyword>
tokens from the answer. The official Q&A pages (https://world.digimoncard.com/rule/?card_no=<ID>)
carry every ruling in full.

This importer reads a crawl of those pages. Today's input is the Aegis simulator's
data/kb/qa.json (MIT, https://github.com/vinicius3333/aegis-digimon-tcg), which is a
transport only: the rulings are Bandai's text, and Aegis's own interpretations (e.g.
data/kb/rule-obligations.json: statuses, branches, expected results) are never read.

An entry is imported only if it traces to the official source:
  - the crawl's manifest names world.digimoncard.com/rule/ as the Q&A source;
  - the entry has exactly the official fields (Q-number "Q<digits>", date, question, answer,
    related card IDs) — any other field is dropped and reported, never imported;
  - a Q-number carries the same question and answer on every card it is listed under.

Output: data/card_official_qa.json, keyed by the card page each ruling is listed on, each ruling
with that page's URL, the crawl it came through, and `about`: the card the ruling is about. A
ruling about one card that names another is listed on both pages; `about` is derived from that
cross-listing (see select_rulings), not taken from the crawl. With --bundles, each data/card_bundles/<ID>.md's "## Official Q&A"
section is rewritten from it (cards the crawl lacks keep their existing section).

Usage:
    python code/tools/import_official_qa.py --kb D:/tmp_aegis/a/data/kb \
        --via "aegis-digimon-tcg@ec0cd22f0 data/kb/qa.json" [--bundles]
"""
import argparse
import json
import os
import re
import sys
from urllib.parse import urlparse

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OFFICIAL_HOST = "world.digimoncard.com"
OFFICIAL_QA_URL = "https://world.digimoncard.com/rule/?card_no={}"
OUT_PATH = os.path.join(ROOT, "data", "card_official_qa.json")
BUNDLE_DIR = os.path.join(ROOT, "data", "card_bundles")

FIELDS = ("qno", "date", "question", "answer", "related")
_QNO = re.compile(r"^Q\d+$")
_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
_CARD_ID = re.compile(r"^[A-Z]+\d*-\d+$")


class ProvenanceError(Exception):
    pass


def check_manifest(manifest):
    """The crawl must name the official Q&A pages as its source."""
    url = (manifest.get("qa") or {}).get("url", "")
    u = urlparse(url)
    if u.scheme != "https" or u.hostname != OFFICIAL_HOST or not u.path.startswith("/rule/"):
        raise ProvenanceError(f"Q&A source is not the official Bandai rule pages: {url!r}")
    return manifest["qa"]


def _valid(e):
    return (isinstance(e.get("qno"), str) and _QNO.match(e["qno"])
            and isinstance(e.get("date"), str) and _DATE.match(e["date"])
            and isinstance(e.get("question"), str) and e["question"].strip()
            and isinstance(e.get("answer"), str) and e["answer"].strip()
            and isinstance(e.get("related"), list)
            and all(isinstance(r, str) and _CARD_ID.match(r) for r in e["related"]))


def select_rulings(qa):
    """Return ({card: [ruling]}, report). Only official-shaped, self-consistent entries survive."""
    report = {"rejected": [], "dropped_fields": {}, "inconsistent_qnos": []}
    texts = {}
    for cid, entries in qa.items():
        for e in entries:
            if _valid(e):
                texts.setdefault(e["qno"], set()).add((e["question"].strip(), e["answer"].strip()))
    bad_q = {q for q, t in texts.items() if len(t) > 1}
    report["inconsistent_qnos"] = sorted(bad_q)

    # A ruling about card S that names card P is listed on both cards' official pages, but on
    # P's page it sits under S's heading. The crawl keeps only the page it came from and drops
    # that page's own ID from `related`, so the copy on S's page is the one whose `related` names
    # every other page it is listed on. `about` records S; a single-page ruling is about its page.
    pages = {}
    for cid, entries in qa.items():
        for e in entries:
            if _valid(e):
                pages.setdefault(e["qno"], {})[cid] = e["related"]
    about = {}
    for qno, m in pages.items():
        subj = [c for c, rel in m.items() if all(d in rel for d in m if d != c)]
        about[qno] = subj[0] if len(subj) == 1 else None
    report["unresolved_subject"] = sorted(q for q, s in about.items() if s is None)

    out = {}
    for cid in sorted(qa):
        if not _CARD_ID.match(cid):
            report["rejected"].append({"card": cid, "why": "card ID shape"})
            continue
        for e in qa[cid]:
            if not _valid(e):
                report["rejected"].append({"card": cid, "qno": e.get("qno"), "why": "not official-shaped"})
                continue
            if e["qno"] in bad_q:
                report["rejected"].append({"card": cid, "qno": e["qno"], "why": "inconsistent text across cards"})
                continue
            extra = sorted(set(e) - set(FIELDS))
            for k in extra:
                report["dropped_fields"][k] = report["dropped_fields"].get(k, 0) + 1
            out.setdefault(cid, []).append({
                "qno": e["qno"], "date": e["date"],
                "question": e["question"].strip(), "answer": e["answer"].strip(),
                "related": list(e["related"]),
                "about": about.get(e["qno"]),
                "source_url": OFFICIAL_QA_URL.format(cid),
            })
    return out, report


def build_sidecar(rulings, qa_manifest, via):
    return {
        "_provenance": {
            "source": "Bandai official card Q&A — " + OFFICIAL_QA_URL.format("<card_no>"),
            "via": via,
            "crawl_fetched_at": qa_manifest.get("fetchedAt"),
            "crawl_scoped_refreshes": [
                {"scope": r.get("scope"), "fetchedAt": r.get("fetchedAt")}
                for r in qa_manifest.get("scopedRefreshes", [])
            ],
            "crawl_failed_cards": qa_manifest.get("failed", []),
            "note": "Rulings are Bandai's text. Only official-shaped entries are imported; the "
                    "crawler's own interpretations are never imported. `about` is derived here "
                    "from how a ruling is cross-listed on the official pages (the card it is "
                    "about; null if unresolved); `source_url` is the page it was read from. "
                    "Regenerate with code/tools/import_official_qa.py.",
        },
        "count": sum(len(v) for v in rulings.values()),
        "cards": rulings,
    }


def _one_line(s):
    return re.sub(r"\s*\n\s*", " ", s).strip()


def render_qa_section(card_id, rulings):
    """Markdown for a bundle's '## Official Q&A' section."""
    lines = ["## Official Q&A",
             f"_Source: {OFFICIAL_QA_URL.format(card_id)} (via data/card_official_qa.json)_"]
    for r in rulings:
        rel = f" _(related: {', '.join(r['related'])})_" if r["related"] else ""
        if r.get("about") and r["about"] != card_id:
            rel = f" _(a ruling on {r['about']}; this card is named in it)_"
        elif r.get("about") is None:
            rel += " _(subject card unresolved)_"
        lines.append(f"- **{r['qno']}** ({r['date']}) Q: {_one_line(r['question'])} "
                     f"— A: {_one_line(r['answer'])}{rel}")
    return lines


_SECTION = re.compile(r"## Official Q&A\n.*?(?=\n## |\Z)", re.S)


def rewrite_bundle_text(text, card_id, rulings):
    """Replace (or insert before '## Image') the bundle's Official Q&A section."""
    block = "\n".join(render_qa_section(card_id, rulings)) + "\n"
    if _SECTION.search(text):
        return _SECTION.sub(lambda _m: block, text, count=1)
    if "\n## Image" in text:
        return text.replace("\n## Image", "\n" + block + "\n## Image", 1)
    return text.rstrip("\n") + "\n\n" + block


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--kb", required=True, help="crawl dir holding qa.json + manifest.json")
    ap.add_argument("--via", required=True, help="where the crawl came from (repo@commit path)")
    ap.add_argument("--out", default=OUT_PATH)
    ap.add_argument("--bundles", action="store_true", help="also rewrite data/card_bundles Q&A sections")
    ap.add_argument("--bundle-dir", default=BUNDLE_DIR)
    args = ap.parse_args(argv)

    manifest = json.load(open(os.path.join(args.kb, "manifest.json"), encoding="utf-8"))
    qa_manifest = check_manifest(manifest)
    qa = json.load(open(os.path.join(args.kb, "qa.json"), encoding="utf-8"))
    rulings, report = select_rulings(qa)
    sidecar = build_sidecar(rulings, qa_manifest, args.via)
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(sidecar, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"# {sidecar['count']} rulings on {len(rulings)} cards -> {args.out}; "
          f"rejected {len(report['rejected'])}, dropped fields {report['dropped_fields']}, "
          f"inconsistent Q-numbers {len(report['inconsistent_qnos'])}, "
          f"unresolved subjects {len(report['unresolved_subject'])}", file=sys.stderr)

    if args.bundles:
        n = 0
        for cid, rs in rulings.items():
            p = os.path.join(args.bundle_dir, f"{cid}.md")
            if not os.path.exists(p):
                continue
            old = open(p, encoding="utf-8").read()
            new = rewrite_bundle_text(old, cid, rs)
            if new != old:
                open(p, "w", encoding="utf-8").write(new)
                n += 1
        print(f"# rewrote the Official Q&A section of {n} bundles", file=sys.stderr)
    return report


if __name__ == "__main__":
    main()
