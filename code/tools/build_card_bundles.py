#!/usr/bin/env python3
"""Build authoritative per-card resource bundles for agents.

Each bundle consolidates, for one card, the *authoritative* data agents need to
implement the card, write tests, and build real-card scenarios — without
re-scraping or relying on the unreliable card-image vision or the lossy
digimoncard.io API:

  - **Digivolution requirements** (standard cost circles AND the "Special
    Digivolution Condition" / DNA / alt-digivolve black text) — from the
    OFFICIAL Bandai DB (`world.digimoncard.com`), the publisher's source of
    truth. The API drops the second colour of multi-colour digivolve lines;
    the official DB lists every one.
  - **Printed text** (effect / inherited / security) — official.
  - **Stats + identity** (name, level, DP, play cost, colours, traits) — merged
    with `data/cards.json` (reliable for these).
  - **Image path** — the local mirror (for optional visual confirmation).
  - **Official Q&A rulings** when present.

Output: `data/card_bundles/<ID>.md` (one human+agent readable file per card) and
a consolidated `data/card_official.json` (the authoritative digivolution data,
machine-readable, suitable for diffing/applying to cards.json overrides).

Usage:
    python code/tools/build_card_bundles.py --ids BT13-020 ST9-05
    python code/tools/build_card_bundles.py --ids-file ids.txt
"""
import argparse
import json
import os
import re
import sys
import time
import urllib.request

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
IMG_DIR = os.environ.get(
    "DIGIMON_CARD_IMAGE_DIR",
    r"C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card",
)
BASE = "https://world.digimoncard.com/cardlist/?search=true&card_no="
UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"
COLOR_CODE = {"red": 0, "blue": 1, "yellow": 2, "green": 3, "white": 4, "black": 5, "purple": 6}
COLOR_NAME = {v: k.capitalize() for k, v in COLOR_CODE.items()}

_PAIR = re.compile(r'cardInfoTit[^>]*>(.*?)</dt>\s*<dd[^>]*>(.*?)</dd>', re.S)
_DIGI_BLOCK = re.compile(r"Digivolve Cost \d+</dt>\s*<dd[^>]*>(.*?)</dd>", re.S)
_COLOR_SPAN = re.compile(r"cardColor_(\w+)'>")
_COST_LV = re.compile(r"(\d+)\s*from\s*Lv\.(\d+)")


def _clean(html: str) -> str:
    return re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", html)).strip()


def fetch(card_id: str, retries: int = 3) -> str:
    req = urllib.request.Request(BASE + card_id, headers={"User-Agent": UA})
    last = None
    for _ in range(retries):
        try:
            return urllib.request.urlopen(req, timeout=30).read().decode("utf-8", "ignore")
        except Exception as e:  # noqa: BLE001
            last = e
            time.sleep(1.5)
    raise last


def parse_evo_costs(html: str):
    """Authoritative standard digivolve-cost circles: distinct (color,level,cost)."""
    seen = []
    for m in _DIGI_BLOCK.finditer(html):
        seg = m.group(1)
        colors = [COLOR_CODE[c.lower()] for c in _COLOR_SPAN.findall(seg) if c.lower() in COLOR_CODE]
        cm = _COST_LV.search(re.sub(r"<[^>]+>", " ", seg))
        if not cm:
            continue
        cost, level = int(cm.group(1)), int(cm.group(2))
        for col in colors:
            t = {"card_color": col, "level": level, "memory_cost": cost}
            if t not in seen:
                seen.append(t)
    return seen


def parse_official(card_id: str, html: str) -> dict:
    """All authoritative fields from the official card page."""
    out = {"card_id": card_id, "digivolve_costs": parse_evo_costs(html)}
    text_sections = []
    for m in _PAIR.finditer(html):
        tit = _clean(m.group(1))
        data = _clean(m.group(2))
        if not data:
            continue
        low = tit.lower()
        if low.startswith("color"):
            out["colors"] = data
        elif low == "cost":
            out["play_cost"] = data
        elif low == "dp":
            out["dp"] = data
        elif low == "form":
            out["form"] = data
        elif low == "attribute":
            out["attribute"] = data
        elif low == "type":
            out["type"] = data
        elif low.startswith("card text"):
            # tit carries the sub-label, e.g. "Card Text 1 [Special Digivolution Condition]"
            label = re.search(r"\[(.*?)\]\s*$", tit)
            text_sections.append({"label": label.group(1) if label else tit, "text": data})
        elif low.startswith("[") or low.endswith("effect]") or low in ("[effect]", "[security]", "[inherited effect]"):
            text_sections.append({"label": tit.strip("[]"), "text": data})
        elif low.startswith("card q"):
            out.setdefault("qa", []).append(data)
    # The official search page repeats a card's block once per printing/foil —
    # keep the FIRST (most-recent printing) text per label. Each label's text is
    # a card's complete clause set, so first-per-label gives one clean copy
    # (later printings only differ by minor errata wording).
    seen_labels = set()
    deduped = []
    for s in text_sections:
        if s["label"] in seen_labels:
            continue
        seen_labels.add(s["label"])
        deduped.append(s)
    out["text_sections"] = deduped
    if "qa" in out:
        out["qa"] = list(dict.fromkeys(out["qa"]))
    # Pull the special digivolution condition out for convenience.
    for s in deduped:
        if "special digivolution" in s["label"].lower():
            out["special_digivolution_condition"] = s["text"]
    return out


def write_bundle(card_id: str, official: dict, meta: dict, outdir: str):
    """Write a per-card markdown bundle merging official + cards.json + image."""
    img = os.path.join(IMG_DIR, card_id + ".webp")
    evo = official.get("digivolve_costs") or []
    evo_lines = [
        f"  - {COLOR_NAME.get(e['card_color'], e['card_color'])} Lv.{e['level']} / cost {e['memory_cost']}"
        for e in evo
    ] or ["  - (none — DigiEgg / Lv.2 base, or DNA/special only)"]
    lines = [
        f"# {meta.get('card_name_eng', card_id)}  ({card_id})",
        "",
        "> Authoritative resource. Digivolution data + text are from the OFFICIAL Bandai",
        "> card DB (world.digimoncard.com). Stats/level from data/cards.json. Image is the",
        "> local mirror. Prefer this over cards.json (lossy API ingest) and over reading the",
        "> card image (unreliable for small digivolve circles).",
        "",
        "## Identity",
        f"- Colors: {official.get('colors', meta.get('card_colors'))}",
        f"- Level: {meta.get('level')}",
        f"- DP: {official.get('dp', meta.get('dp'))}",
        f"- Play cost: {official.get('play_cost', meta.get('play_cost'))}",
        f"- Type/Traits: {official.get('type', '')}",
        f"- Form: {official.get('form', '')}   Attribute: {official.get('attribute', '')}",
        "",
        "## Digivolution requirements (AUTHORITATIVE — official Bandai DB)",
        "### Standard digivolve cost circles",
        *evo_lines,
    ]
    if official.get("special_digivolution_condition"):
        lines += [
            "### Special digivolution condition (DNA / alt-digivolve black text)",
            f"  {official['special_digivolution_condition']}",
        ]
    lines += ["", "## Printed text (official)"]
    for s in official.get("text_sections", []):
        if "special digivolution" in s["label"].lower():
            continue
        lines += [f"### {s['label']}", s["text"], ""]
    if official.get("qa"):
        lines += ["## Official Q&A"] + [f"- {q}" for q in official["qa"]]
    lines += [
        "",
        "## Image",
        f"- {img}" + ("" if os.path.exists(img) else "  (NOT in local mirror)"),
        "",
        f"_Source: {BASE}{card_id}_",
    ]
    open(os.path.join(outdir, f"{card_id}.md"), "w", encoding="utf-8").write("\n".join(lines))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ids", nargs="*", default=[])
    ap.add_argument("--ids-file", default=None)
    ap.add_argument("--outdir", default=os.path.join(ROOT, "data", "card_bundles"))
    ap.add_argument("--delay", type=float, default=0.4)
    args = ap.parse_args()

    ids = list(args.ids)
    if args.ids_file:
        ids += [l.strip() for l in open(args.ids_file, encoding="utf-8") if l.strip()]
    ids = list(dict.fromkeys(ids))

    cards = json.loads(open(os.path.join(ROOT, "data", "cards.json"), encoding="utf-8").read())
    os.makedirs(args.outdir, exist_ok=True)
    official_index = {}
    failed = []
    for i, cid in enumerate(ids):
        try:
            official = parse_official(cid, fetch(cid))
            official_index[cid] = official
            write_bundle(cid, official, cards.get(cid, {}), args.outdir)
        except Exception as e:  # noqa: BLE001
            failed.append(cid)
            print(f"# FAILED {cid}: {type(e).__name__} {e}", file=sys.stderr)
        if (i + 1) % 25 == 0:
            print(f"# {i + 1}/{len(ids)} bundled", file=sys.stderr)
        time.sleep(args.delay)

    idx_path = os.path.join(ROOT, "data", "card_official.json")
    json.dump(
        {"count": len(official_index), "failed": failed, "cards": official_index},
        open(idx_path, "w", encoding="utf-8"), ensure_ascii=False, indent=2,
    )
    print(f"# {len(official_index)} bundles -> {args.outdir}; index -> {idx_path} ({len(failed)} failed)", file=sys.stderr)


if __name__ == "__main__":
    main()
