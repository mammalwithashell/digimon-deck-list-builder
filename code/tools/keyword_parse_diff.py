#!/usr/bin/env python3
"""Pool-wide innate-keyword parse diff (fix-innate-keyword-overextraction).

Compares the OLD innate-keyword extraction (scan EVERY <...> token in a card's
effect/inherited/security text) against the NEW grammar-based extraction (only
the LEADING keyword line, before the first [Timing]/prose), for every card in
data/cards.json. Emits the per-card delta and partitions cards that LOSE a
keyword into implemented (a DSL spec exists under code/digimon-engine/cards/)
vs unimplemented.

The "loss" set is the audit worklist: implemented losers must have the grant
modeled as a DSL effect; unimplemented losers get logged to RUST_ENGINE_GAPS.

Usage: python code/tools/keyword_parse_diff.py [--out <path.md>]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CARDS = REPO / "data" / "cards.json"
DSL_CARDS = REPO / "code" / "digimon-engine" / "cards"

HEADERS = ["Inherited Effect", "Security Effect", "Rule Effect"]

# Token prefixes the Rust classifier recognizes as keywords (card_data.rs).
KEYWORD_PREFIXES = [
    "Blast Digivolve", "Blocker", "Rush", "Jamming", "Piercing", "Reboot",
    "Blitz", "Armor Purge", "Raid", "Alliance", "Save", "Fortitude",
    "Overclock", "Barrier", "Evade", "Decode", "Partition", "Vortex",
    "Collision", "Progress", "Retaliation", "Scapegoat", "Mind Link",
    "Iceclad", "Execute", "Training", "Arts Digivolve", "Ascension",
    "Security A.", "De-Digivolve", "Draw", "Material Save", "Digi-Burst",
    "Decoy", "Fragment",
]


def is_keyword(token: str) -> bool:
    return any(token.startswith(p) for p in KEYWORD_PREFIXES)


def strip_leading_headers(text: str) -> str:
    s = text.lstrip()
    for h in HEADERS:
        if s.startswith(h):
            return s[len(h):].lstrip()
    return s


def consume_balanced_parens(s: str) -> str:
    """s starts with '('. Return slice after matching ')'."""
    depth = 0
    for i, c in enumerate(s):
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return s[i + 1:]
    return ""


def all_tokens(text: str) -> list[str]:
    """OLD behavior: every <...> token anywhere."""
    toks, s = [], text
    while True:
        a = s.find("＜")
        if a == -1:
            break
        b = s.find("＞", a + 1)
        if b == -1:
            break
        toks.append(s[a + 1:b].strip())
        s = s[b + 1:]
    return toks


def leading_tokens(text: str) -> list[str]:
    """NEW behavior: innate tokens by LEFT CONTEXT (mirrors the Rust
    context-classifier in card_data.rs). A ＜kw＞ is innate when, after
    stripping the leading header, its left context (trimmed of trailing
    whitespace) is empty, or ends with `]` (a [Timing/Location] label),
    `)` (a prior keyword's reminder), or `＞` (a chained keyword)."""
    s = strip_leading_headers(text)
    out = []
    i = 0
    while True:
        a = s.find("＜", i)
        if a == -1:
            break
        b = s.find("＞", a + 1)
        if b == -1:
            break
        inside = s[a + 1:b].strip()
        i = b + 1
        left = s[:a].rstrip()
        if left == "" or left.endswith("]") or left.endswith(")") or left.endswith("＞"):
            out.append(inside)
    return out


def keyword_set(extractor, fields: list[str]) -> set[str]:
    out: set[str] = set()
    for f in fields:
        for t in extractor(f or ""):
            if is_keyword(t):
                out.add(t)
    return out


def has_dsl_spec(card_id: str) -> bool:
    # DSL implementations are *.yaml. The *.json files in this tree are per-card
    # METADATA (ingest), NOT implementations — do not count them.
    set_id = card_id.split("-")[0].lower()
    return (DSL_CARDS / set_id / f"{card_id}.yaml").exists()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, default=REPO / "openspec" / "changes"
                    / "fix-innate-keyword-overextraction" / "keyword-diff.md")
    args = ap.parse_args()

    cards = json.loads(CARDS.read_text(encoding="utf-8"))
    items = list(cards.values()) if isinstance(cards, dict) else cards

    impl_losers: list[dict] = []
    unimpl_losers: list[dict] = []
    gained: list[dict] = []  # NEW finds something OLD didn't (should be ~none)

    for c in items:
        cid = c.get("card_id")
        if not cid:
            continue
        fields = [
            c.get("effect_description_eng") or "",
            c.get("inherited_effect_description_eng") or "",
            c.get("security_effect_description_eng") or "",
        ]
        old = keyword_set(all_tokens, fields)
        new = keyword_set(leading_tokens, fields)
        lost = sorted(old - new)
        extra = sorted(new - old)
        if extra:
            gained.append({"card_id": cid, "gained": extra})
        if lost:
            row = {"card_id": cid, "name": c.get("card_name_eng"), "lost": lost}
            (impl_losers if has_dsl_spec(cid) else unimpl_losers).append(row)

    # Aggregate which keywords are most affected.
    from collections import Counter
    impl_kw = Counter(k for r in impl_losers for k in r["lost"])

    lines = ["# Innate-keyword parse diff", "",
             f"- cards scanned: {len(items)}",
             f"- cards losing >=1 keyword: {len(impl_losers) + len(unimpl_losers)}",
             f"- **implemented (DSL) losers: {len(impl_losers)}**  <- Step 3 worklist",
             f"- unimplemented losers: {len(unimpl_losers)}",
             f"- cards where NEW gained a token OLD missed: {len(gained)} (should be ~0)",
             "", "## Implemented-loser keyword histogram", ""]
    for kw, n in impl_kw.most_common():
        lines.append(f"- `{kw}`: {n}")
    lines += ["", "## Implemented losers (Step 3 worklist)", ""]
    for r in sorted(impl_losers, key=lambda r: r["card_id"]):
        lines.append(f"- {r['card_id']} {r['name']}: {', '.join(r['lost'])}")
    if gained:
        lines += ["", "## NEW-gained (investigate)", ""]
        for r in gained:
            lines.append(f"- {r['card_id']}: {', '.join(r['gained'])}")
    lines += ["", "## Unimplemented losers (log to RUST_ENGINE_GAPS)", ""]
    for r in sorted(unimpl_losers, key=lambda r: r["card_id"]):
        lines.append(f"- {r['card_id']} {r['name']}: {', '.join(r['lost'])}")

    args.out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"implemented_losers={len(impl_losers)} "
          f"unimplemented_losers={len(unimpl_losers)} "
          f"new_gained={len(gained)}")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
