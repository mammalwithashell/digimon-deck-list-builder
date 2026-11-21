#!/usr/bin/env python3
"""Authoritative digivolution-cost source: scrape world.digimoncard.com.

The digimoncard.io API (which `cards.json` is ingested from) DROPS the second
colour of a multi-colour digivolution requirement (e.g. it reports EX1-014
ExVeemon as "Blue Lv.3" when the card actually digivolves from a Blue OR Green
Lv.3). Card-image vision is also unreliable at the small circle sizes. The
OFFICIAL Bandai card database is the publisher's authoritative, structured
source and lists every colour of every digivolve-cost line:

    <dt>Digivolve Cost 1</dt>
    <dd class="cardColorCost">
      <span class='cardColor_red'>Red</span><span class='cardColor_yellow'>Yellow</span>
      5 from Lv.6
    </dd>

That parses to evo_costs [{Red, Lv6, 5}, {Yellow, Lv6, 5}].

This tool fetches each card's official page, parses every "Digivolve Cost N"
block into evo_costs (one entry per colour in the line), and writes a mapping
`{card_id: [evo_costs]}`. A companion step diffs this against `data/cards.json`
and emits the corrections.

Usage:
    python code/tools/scrape_official_evo_costs.py --ids BT13-020 EX1-014
    python code/tools/scrape_official_evo_costs.py --ids-file ids.txt --out evo.json
"""
import argparse
import json
import re
import sys
import time
import urllib.request

COLOR_CODE = {
    "red": 0, "blue": 1, "yellow": 2, "green": 3,
    "white": 4, "black": 5, "purple": 6,
}
BASE = "https://world.digimoncard.com/cardlist/?search=true&card_no="
UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"

# One "Digivolve Cost N" definition list block.
_BLOCK = re.compile(r"Digivolve Cost \d+</dt>\s*<dd[^>]*>(.*?)</dd>", re.S)
_COLOR_SPAN = re.compile(r"cardColor_(\w+)'>")
_COST_LV = re.compile(r"(\d+)\s*from\s*Lv\.(\d+)")


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


def _circle_colors(seg: str):
    """Colour codes of one digivolve-cost row, in printed order.

    ``<span class='cardColor_all'>Multicolor</span>`` is the rainbow circle: ANY
    colour (JP DB 「全」 = all; DCGO has one EvoCosts row per colour), so it expands
    to all seven. It used to be dropped, which made a rainbow-circle card look as if
    it printed no circle. Same rule as `build_card_bundles._circle_colors`.
    """
    colors = []
    for name in _COLOR_SPAN.findall(seg):
        name = name.lower()
        codes = sorted(COLOR_CODE.values()) if name == "all" else [COLOR_CODE[name]] if name in COLOR_CODE else []
        for code in codes:
            if code not in colors:
                colors.append(code)
    return colors


def parse_evo_costs(html: str):
    """Distinct (card_color, level, memory_cost) tuples from the official page.

    The search page repeats a card's block once per printing/foil; tuples are
    de-duplicated. Genuinely distinct cost lines (different level/cost) are kept.
    Order is preserved (first-seen).
    """
    seen = []
    for m in _BLOCK.finditer(html):
        seg = m.group(1)
        colors = _circle_colors(seg)
        text = re.sub(r"<[^>]+>", " ", seg)
        cm = _COST_LV.search(text)
        if not cm:
            continue
        cost, level = int(cm.group(1)), int(cm.group(2))
        for col in colors:
            t = {"card_color": col, "level": level, "memory_cost": cost}
            if t not in seen:
                seen.append(t)
    return seen


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ids", nargs="*", default=[])
    ap.add_argument("--ids-file", default=None)
    ap.add_argument("--out", default=None)
    ap.add_argument("--delay", type=float, default=0.4)
    args = ap.parse_args()

    ids = list(args.ids)
    if args.ids_file:
        ids += [l.strip() for l in open(args.ids_file, encoding="utf-8") if l.strip()]
    ids = list(dict.fromkeys(ids))  # de-dupe, preserve order

    result = {}
    failed = []
    for i, cid in enumerate(ids):
        try:
            evo = parse_evo_costs(fetch(cid))
            result[cid] = evo
        except Exception as e:  # noqa: BLE001
            failed.append(cid)
            print(f"# FAILED {cid}: {type(e).__name__} {e}", file=sys.stderr)
        if (i + 1) % 25 == 0:
            print(f"# {i + 1}/{len(ids)} scraped", file=sys.stderr)
        time.sleep(args.delay)

    payload = {"count": len(result), "failed": failed, "evo_costs": result}
    out = json.dumps(payload, ensure_ascii=False, indent=2)
    if args.out:
        open(args.out, "w", encoding="utf-8").write(out)
        print(f"# wrote {len(result)} cards to {args.out} ({len(failed)} failed)", file=sys.stderr)
    else:
        sys.stdout.write(out)


if __name__ == "__main__":
    main()
