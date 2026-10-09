#!/usr/bin/env python3
"""Scrape the official Q&A rulings from the Bandai card DB into `data/card_qa.json`.

Each ruling on `world.digimoncard.com` carries a stable number (`Q1601`), an
optional date, a question and the publisher's answer. They are the most
authoritative statement of how two cards / effects interact, so the card loop
uses them as interaction-exam specs with an expected answer attached.

Why a separate dataset instead of `card_official.json`'s `qa` field: that field
came from `build_card_bundles.py`, whose `cardInfoTit … </dt><dd> … </dd>` match
swallowed the first QUESTION into the section title and kept only the first
ANSWER per card (2,590 cards, exactly one answer each). This scraper parses the
`cardFaqListItem` markup directly and keys rulings by Q-number, so a ruling
printed on several cards (reprints, shared rulings) is stored once.

Output shape (`version: 1`, keys sorted for stable diffs):

    cards     {card_id: [q_id, ...]}        every card fetched and found (may be [])
    qa        {q_id: {q_id, date, question, answer, card_ids}}
    failed    [card_id, ...]                search found no such card, or fetch/parse error
    conflicts [{q_id, card_id}, ...]        one Q-number with different text on another card

Usage:
    python -m tools.scrape_official_qa --ids BT7-056 ST1-01
    python -m tools.scrape_official_qa --all              # resumes: skips cards already scraped
    python -m tools.scrape_official_qa --all --refresh    # re-fetch everything (new rulings)
"""
import argparse
import html
import json
import os
import re
import sys
import time
import urllib.request
from collections import defaultdict
from dataclasses import dataclass, field

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
# `/cardlist/` (build_card_bundles.py's BASE) now 301-redirects here.
SEARCH_URL = "https://world.digimoncard.com/cards/?search=true&card_no="
UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"
DEFAULT_OUT = os.path.join(ROOT, "data", "card_qa.json")

_CARD_NO = re.compile(r'<li class="cardNo\b[^"]*"[^>]*>(.*?)</li>', re.S)
_ITEM_START = re.compile(r'<li class="cardFaqListItem\b[^"]*"[^>]*>')
_FIELDS = {
    "q_id": re.compile(r'class="cardFaqNum\b[^"]*"[^>]*>(.*?)</p>', re.S),
    "date": re.compile(r'class="cardFaqDate\b[^"]*"[^>]*>(.*?)</p>', re.S),
    "question": re.compile(r'class="cardFaqQuestion\b[^"]*"[^>]*>(.*?)</dt>', re.S),
    "answer": re.compile(r'class="cardFaqAnswer\b[^"]*"[^>]*>(.*?)</dd>', re.S),
}
# Tags that separate words become a space; inline tags (<b>, <span>) vanish so a
# word split by markup is not split in the text.
_BREAK_TAG = re.compile(r"<\s*(?:br|/p|/div|/li|/dt|/dd)\b[^>]*>", re.I)
_TAG = re.compile(r"<[^>]+>")
_DATE = re.compile(r"([A-Za-z]+)\.?\s+(\d{1,2}),\s*(\d{4})")
_MONTHS = {m: i for i, m in enumerate(
    ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"], 1)}


@dataclass
class PageResult:
    found: bool
    entries: list = field(default_factory=list)


def _text(fragment: str) -> str:
    s = _TAG.sub("", _BREAK_TAG.sub(" ", fragment))
    return re.sub(r"\s+", " ", html.unescape(s)).strip()


def parse_date(raw: str) -> str:
    """`Mar. 28, 2024` / `Sept. 3, 2025` -> ISO date; anything else is kept raw."""
    m = _DATE.fullmatch(raw.strip())
    if m and m.group(1)[:3].lower() in _MONTHS:
        return f"{int(m.group(3)):04d}-{_MONTHS[m.group(1)[:3].lower()]:02d}-{int(m.group(2)):02d}"
    return raw.strip()


def _q_sort_key(q_id: str):
    digits = re.sub(r"\D", "", q_id)
    return (int(digits) if digits else 0, q_id)


def _parse_item(chunk: str) -> dict:
    out = {}
    for name in ("q_id", "question", "answer"):
        m = _FIELDS[name].search(chunk)
        if not m or not _text(m.group(1)):
            raise ValueError(f"Q&A item missing {name}: {_text(chunk)[:120]!r}")
        out[name] = _text(m.group(1))
    m = _FIELDS["date"].search(chunk)
    date = _text(m.group(1)) if m else ""
    return {"q_id": out["q_id"], "date": parse_date(date) if date else None,
            "question": out["question"], "answer": out["answer"]}


def parse_qa_page(card_id: str, page: str) -> PageResult:
    """Rulings printed for `card_id` on its official search page.

    The page holds one block per printing, each opened by a `cardNo` element.
    Only blocks whose card number is exactly `card_id` contribute; a page with
    none is `found=False` (a no-results page is not "this card has no Q&A").
    Rulings repeat once per printing and are deduplicated by Q-number.
    """
    marks = [(m.start(), _text(m.group(1))) for m in _CARD_NO.finditer(page)]
    segments = [
        page[start:(marks[i + 1][0] if i + 1 < len(marks) else len(page))]
        for i, (start, card_no) in enumerate(marks) if card_no == card_id
    ]
    if not segments:
        return PageResult(found=False)
    entries, seen = [], set()
    for seg in segments:
        items = list(_ITEM_START.finditer(seg))
        for i, m in enumerate(items):
            end = items[i + 1].start() if i + 1 < len(items) else len(seg)
            entry = _parse_item(seg[m.end():end])
            if entry["q_id"] not in seen:
                seen.add(entry["q_id"])
                entries.append(entry)
    return PageResult(found=True, entries=entries)


def new_dataset(existing: dict | None = None) -> dict:
    """Working state; `existing` is a previously written `card_qa.json`."""
    existing = existing or {}
    return {
        "cards": {c: list(qs) for c, qs in existing.get("cards", {}).items()},
        "qa": {q: {k: v for k, v in e.items() if k != "card_ids"}
               for q, e in existing.get("qa", {}).items()},
        "failed": set(existing.get("failed", [])),
        "conflicts": [dict(c) for c in existing.get("conflicts", [])],
    }


def merge_page(ds: dict, card_id: str, result: PageResult) -> None:
    if not result.found:
        # Keep any previously scraped rulings: a bad page is not evidence they vanished.
        ds["failed"].add(card_id)
        return
    ds["failed"].discard(card_id)
    for entry in result.entries:
        q_id = entry["q_id"]
        prior = ds["qa"].get(q_id)
        other_owners = [c for c, qs in ds["cards"].items() if c != card_id and q_id in qs]
        changed = prior is not None and (prior["question"], prior["answer"]) != (
            entry["question"], entry["answer"])
        if changed and other_owners:
            conflict = {"q_id": q_id, "card_id": card_id}
            if conflict not in ds["conflicts"]:
                ds["conflicts"].append(conflict)
            continue
        ds["qa"][q_id] = dict(entry)
    ds["cards"][card_id] = [e["q_id"] for e in result.entries]


def finalize(ds: dict) -> dict:
    cards = {c: sorted(qs, key=_q_sort_key) for c, qs in sorted(ds["cards"].items())}
    owners = defaultdict(list)
    for c, qs in cards.items():
        for q in qs:
            owners[q].append(c)
    qa = {q: {**ds["qa"][q], "card_ids": sorted(owners[q])}
          for q in sorted(owners, key=_q_sort_key) if q in ds["qa"]}
    return {
        "version": 1,
        "source": SEARCH_URL,
        "card_count": len(cards),
        "qa_count": len(qa),
        "cards": cards,
        "qa": qa,
        "failed": sorted(ds["failed"]),
        "conflicts": sorted(ds["conflicts"], key=lambda c: (_q_sort_key(c["q_id"]), c["card_id"])),
    }


def fetch(card_id: str, retries: int = 3) -> str:
    req = urllib.request.Request(SEARCH_URL + card_id, headers={"User-Agent": UA})
    last = None
    for attempt in range(retries):
        try:
            return urllib.request.urlopen(req, timeout=30).read().decode("utf-8", "ignore")
        except Exception as e:  # noqa: BLE001
            last = e
            time.sleep(1.5 * (attempt + 1))
    raise last


def write_dataset(ds: dict, path: str) -> None:
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8", newline="\n") as f:
        json.dump(finalize(ds), f, ensure_ascii=False, indent=2)
        f.write("\n")
    os.replace(tmp, path)


def _all_card_ids() -> list:
    ids = set()
    with open(os.path.join(ROOT, "data", "cards.json"), encoding="utf-8") as f:
        ids.update(json.load(f))
    with open(os.path.join(ROOT, "data", "card_official.json"), encoding="utf-8") as f:
        ids.update(json.load(f).get("cards", {}))
    return sorted(ids)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--ids", nargs="*", default=[])
    ap.add_argument("--ids-file", default=None)
    ap.add_argument("--all", action="store_true",
                    help="every card id in data/cards.json and data/card_official.json")
    ap.add_argument("--refresh", action="store_true",
                    help="re-fetch cards already in the dataset (default: skip them)")
    ap.add_argument("--out", default=DEFAULT_OUT)
    ap.add_argument("--delay", type=float, default=0.5, help="seconds between requests")
    ap.add_argument("--checkpoint", type=int, default=50, help="write the dataset every N cards")
    args = ap.parse_args(argv)

    ids = list(args.ids)
    if args.ids_file:
        with open(args.ids_file, encoding="utf-8") as f:
            ids += [line.strip() for line in f if line.strip()]
    if args.all:
        ids += _all_card_ids()
    ids = list(dict.fromkeys(ids))
    if not ids:
        ap.error("give --ids, --ids-file, or --all")

    existing = None
    if os.path.exists(args.out):
        with open(args.out, encoding="utf-8") as f:
            existing = json.load(f)
    ds = new_dataset(existing)
    todo = ids if args.refresh else [c for c in ids if c not in ds["cards"]]
    print(f"# {len(todo)} to fetch ({len(ids) - len(todo)} already scraped)", file=sys.stderr)

    errors = 0
    for i, cid in enumerate(todo):
        try:
            merge_page(ds, cid, parse_qa_page(cid, fetch(cid)))
        except Exception as e:  # noqa: BLE001
            errors += 1
            ds["failed"].add(cid)
            print(f"# FAILED {cid}: {type(e).__name__} {e}", file=sys.stderr)
        if (i + 1) % args.checkpoint == 0:
            write_dataset(ds, args.out)
            print(f"# {i + 1}/{len(todo)} fetched", file=sys.stderr)
        time.sleep(args.delay)

    write_dataset(ds, args.out)
    out = finalize(ds)
    print(f"# {out['card_count']} cards, {out['qa_count']} rulings, "
          f"{len(out['failed'])} failed ({errors} errors this run), "
          f"{len(out['conflicts'])} conflicts -> {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
