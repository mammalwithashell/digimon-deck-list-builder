#!/usr/bin/env python3
"""Deep audit of the innate-keyword parse diff (fix-innate-keyword-overextraction).

For every implemented card that loses a keyword token under the new leading-line
parser, classify each lost token and decide whether it is a genuine regression
risk:

  role (by left-context of the token):
    - filter   : preceded by with/without/has/have  -> the keyword qualifies a
                 TARGET or CONDITION, never this card's own keyword. SAFE.
    - grant    : preceded by gain/gains, or a DP-and-keyword "..., <kw>" clause.
    - ambiguous: token right after a [Timing] label with no verb. Treated as grant.

  meaningful: behaviorally-checkable keywords. Pure-action keywords (Draw,
    De-Digivolve) are inert as innate attributes -> never a regression.

  modeled: does the card's DSL YAML/JSON already grant the keyword via an effect
    (grant_keyword / security_attack_fn / SecurityAttack)? If yes, the phantom
    innate keyword was redundant and dropping it is safe.

RISK = meaningful AND (grant|ambiguous) AND NOT modeled. That set (plus any
genuinely-innate keyword the parser wrongly dropped) is the human audit surface.

Usage: python code/tools/keyword_audit.py [--out <path.md>]
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CARDS = REPO / "data" / "cards.json"
DSL_CARDS = REPO / "code" / "digimon-engine" / "cards"

from keyword_parse_diff import (  # reuse the exact tokenizers
    all_tokens, leading_tokens, is_keyword, KEYWORD_PREFIXES,
)

# Pure-action keywords: inert as an innate attribute (executed by effects).
INERT_ACTION = ("Draw", "De-Digivolve")

FILTER_CUE = re.compile(r"(with|without|has|have|having)\s*$", re.IGNORECASE)
GRANT_CUE = re.compile(r"(gain|gains)\s*$", re.IGNORECASE)


def keyword_name(token: str) -> str:
    for p in KEYWORD_PREFIXES:
        if token.startswith(p):
            return p
    return token


def lost_tokens_with_context(fields: list[str]):
    """Yield (token, role) for keyword tokens present in OLD-parse but not NEW."""
    new = set()
    for f in fields:
        for t in leading_tokens(f or ""):
            if is_keyword(t):
                new.add(t)
    seen_new = dict(Counter())  # not needed; set membership by exact string
    for f in fields:
        s = f or ""
        idx = 0
        while True:
            a = s.find("＜", idx)
            if a == -1:
                break
            b = s.find("＞", a + 1)
            if b == -1:
                break
            tok = s[a + 1:b].strip()
            idx = b + 1
            if not is_keyword(tok) or tok in new:
                continue
            left = s[max(0, a - 24):a]
            if FILTER_CUE.search(left):
                role = "filter"
            elif GRANT_CUE.search(left):
                role = "grant"
            else:
                role = "ambiguous"
            yield tok, role


def yaml_models(card_id: str, kw_name: str) -> bool:
    # Only the *.yaml DSL spec models behavior (*.json is metadata).
    set_id = card_id.split("-")[0].lower()
    p = DSL_CARDS / set_id / f"{card_id}.yaml"
    if not p.exists():
        return False
    text = p.read_text(encoding="utf-8")
    if kw_name.startswith("Security A."):
        return ("security_attack_fn" in text
                or "SecurityAttack" in text
                or "change_s_attack" in text
                or "ChangeSAttack" in text
                or ("grant_keyword" in text and "Security" in text))
    bare = kw_name.replace(" ", "")
    return "grant_keyword" in text and (kw_name in text or bare in text)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, default=REPO / "openspec" / "changes"
                    / "fix-innate-keyword-overextraction" / "keyword-audit.md")
    args = ap.parse_args()

    cards = json.loads(CARDS.read_text(encoding="utf-8"))
    items = list(cards.values()) if isinstance(cards, dict) else cards

    role_counter: Counter = Counter()
    risk: list[dict] = []

    for c in items:
        cid = c.get("card_id")
        if not cid:
            continue
        set_id = cid.split("-")[0].lower()
        if not (DSL_CARDS / set_id / f"{cid}.yaml").exists():
            continue  # implemented (DSL *.yaml) cards only
        fields = [
            c.get("effect_description_eng") or "",
            c.get("inherited_effect_description_eng") or "",
            c.get("security_effect_description_eng") or "",
        ]
        for tok, role in lost_tokens_with_context(fields):
            kw = keyword_name(tok)
            role_counter[role] += 1
            meaningful = not kw.startswith(INERT_ACTION)
            if not meaningful or role == "filter":
                continue
            if yaml_models(cid, kw):
                continue
            risk.append({"card_id": cid, "name": c.get("card_name_eng"),
                         "keyword": tok, "role": role})

    risk_kw = Counter(r["keyword"] for r in risk)
    risk_cards = sorted({r["card_id"] for r in risk})

    lines = ["# Innate-keyword audit (regression-risk surface)", "",
             f"- role breakdown of lost tokens: {dict(role_counter)}",
             f"- **RISK tokens (meaningful + grant/ambiguous + not modeled): {len(risk)}**",
             f"- **RISK cards (distinct): {len(risk_cards)}**",
             "", "## RISK keyword histogram", ""]
    for kw, n in risk_kw.most_common():
        lines.append(f"- `{kw}`: {n}")
    lines += ["", "## RISK cards", ""]
    for r in sorted(risk, key=lambda r: r["card_id"]):
        lines.append(f"- {r['card_id']} {r['name']}: `{r['keyword']}` ({r['role']})")

    args.out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"risk_tokens={len(risk)} risk_cards={len(risk_cards)} "
          f"roles={dict(role_counter)}")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
