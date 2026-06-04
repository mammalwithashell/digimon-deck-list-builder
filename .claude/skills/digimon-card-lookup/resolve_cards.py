#!/usr/bin/env python3
"""Resolve Digimon card queries -> local image paths + printed-text fields.

Pure *resolution* helper for the `digimon-card-lookup` skill. It does NOT view
images; it emits a manifest of card IDs, the local `.webp` path for each (downloading
from the digimoncard.io CDN when the local mirror is missing one), and the
machine-readable text fields. The assistant then `Read`s the listed `.webp` paths.

Three query scopes (auto-detected, or forced with --type):
  * card ID    e.g. BT20-065            -> that printing (+ alt-art editions with --editions)
  * card name  e.g. Wormmon            -> ALL matching printings (Wormmon = 8 distinct cards)
  * archetype  e.g. "Xros Heart"       -> the unique card pool from deck_library.json

Examples:
  python resolve_cards.py BT17-001
  python resolve_cards.py Wormmon
  python resolve_cards.py BT24-102 --editions
  python resolve_cards.py "Xros Heart" --type archetype
  python resolve_cards.py Wormmon --paths-only        # just the image paths, one per line
  python resolve_cards.py Wormmon --json              # structured manifest
"""
from __future__ import annotations

import argparse
import glob
import json
import os
import re
import sys
import urllib.request
from pathlib import Path

# --- paths -----------------------------------------------------------------

DEFAULT_IMAGE_DIR = r"C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card"
CDN_URL = "https://images.digimoncard.io/images/cards/{cid}.webp"
CARD_ID_RE = re.compile(r"^[A-Z]{1,3}\d{0,2}-\d{1,3}$", re.IGNORECASE)
# IDs as they appear embedded in free text (used by the hook too).
CARD_ID_SCAN_RE = re.compile(r"\b(?:BT|EX|ST|RB|AD|LM|P)\d{0,2}-\d{2,3}\b", re.IGNORECASE)


def repo_root() -> Path:
    """Walk up from this file until we find data/cards.json."""
    here = Path(__file__).resolve()
    for parent in [here, *here.parents]:
        if (parent / "data" / "cards.json").is_file():
            return parent
    raise SystemExit("could not locate repo root (data/cards.json not found above this script)")


def image_dir() -> Path:
    return Path(os.environ.get("DIGIMON_CARD_IMAGE_DIR", DEFAULT_IMAGE_DIR))


def cache_dir() -> Path:
    d = Path(__file__).resolve().parent / ".cache"
    d.mkdir(parents=True, exist_ok=True)
    return d


# --- data loading ----------------------------------------------------------

def load_json(path: Path):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def load_data(root: Path):
    cards = load_json(root / "data" / "cards.json")
    overrides = load_json(root / "data" / "card_overrides.json")
    return cards, overrides


# --- text extraction -------------------------------------------------------

TEXT_FIELDS = {
    "name": "card_name_eng",
    "color": "card_colors",
    "level": "level",
    "play_cost": "play_cost",
    "dp": "dp",
    "type": "type_eng",
    "form": "form_eng",
    "kind": "card_kind",
    "effect": "effect_description_eng",
    "inherited": "inherited_effect_description_eng",
    "security": "security_effect_description_eng",
}


def card_text(cid: str, cards: dict, overrides: dict) -> dict:
    raw = cards.get(cid, {})
    out = {}
    for label, key in TEXT_FIELDS.items():
        out[label] = raw.get(key, "")
    # Apply trusted overrides (sparse; mostly color / evo_costs patches).
    ov = overrides.get(cid)
    if isinstance(ov, dict):
        out["_overrides"] = {k: v for k, v in ov.items() if not k.startswith("_")}
    return out


# --- query resolution ------------------------------------------------------

def index_names(cards: dict) -> dict[str, list[str]]:
    """lower(card_name_eng) -> [card_id, ...] (all printings)."""
    idx: dict[str, list[str]] = {}
    for cid, c in cards.items():
        name = str(c.get("card_name_eng", "")).strip().lower()
        if name:
            idx.setdefault(name, []).append(cid)
    return idx


def index_archetypes(root: Path):
    """canonical/alias archetype name (lower) -> set(card_id)."""
    dl = load_json(root / "data" / "deck_library.json")
    archetypes = dl.get("archetypes", {})
    aliases = load_json(root / "data" / "archetype_aliases.json")

    pools: dict[str, set[str]] = {}
    for name, entry in archetypes.items():
        ids: set[str] = set()
        for deck in entry.get("decklists", []):
            raw = deck.get("decklist", "[]")
            try:
                lst = json.loads(raw) if isinstance(raw, str) else (raw or [])
            except json.JSONDecodeError:
                lst = []
            ids.update(lst)
        pools[name.strip().lower()] = ids

    # Map aliases -> canonical pool.
    alias_map: dict[str, str] = {}
    for canon, alist in aliases.items():
        if canon.startswith("_"):
            continue
        canon_l = canon.strip().lower()
        alias_map[canon_l] = canon_l
        if isinstance(alist, list):
            for a in alist:
                alias_map[str(a).strip().lower()] = canon_l
    return pools, alias_map


def detect_type(q: str, name_idx, arch_pools, alias_map) -> str:
    if CARD_ID_RE.match(q):
        return "id"
    ql = q.strip().lower()
    if ql in name_idx:
        return "name"
    if ql in arch_pools or ql in alias_map:
        return "archetype"
    # fuzzy fallbacks
    if any(ql in n for n in name_idx):
        return "name"
    if any(ql in n for n in arch_pools) or any(ql in a for a in alias_map):
        return "archetype"
    return "unknown"


def resolve_query(q: str, qtype: str, cards, name_idx, arch_pools, alias_map) -> tuple[str, list[str]]:
    """Return (resolved_type, [card_id, ...])."""
    if qtype == "auto":
        qtype = detect_type(q, name_idx, arch_pools, alias_map)
    ql = q.strip().lower()

    if qtype == "id":
        return "id", [q.upper()]
    if qtype == "name":
        if ql in name_idx:
            return "name", sorted(name_idx[ql])
        hits: list[str] = []
        for n, ids in name_idx.items():
            if ql in n:
                hits.extend(ids)
        return "name", sorted(set(hits))
    if qtype == "archetype":
        canon = alias_map.get(ql, ql)
        if canon in arch_pools:
            return "archetype", sorted(arch_pools[canon])
        # contains-match on archetype names
        for n, ids in arch_pools.items():
            if ql in n:
                return "archetype", sorted(ids)
        return "archetype", []
    return "unknown", []


# --- image resolution ------------------------------------------------------

def resolve_image(cid: str, idir: Path, editions: bool, download: bool) -> dict:
    """Return {id, status, paths:[...]} where status is local/downloaded/missing."""
    base = idir / f"{cid}.webp"
    paths: list[str] = []
    status = "missing"
    if base.is_file():
        status = "local"
        paths.append(str(base))
        if editions:
            for p in sorted(glob.glob(str(idir / f"{cid}_P*.webp"))):
                paths.append(p)
    elif download:
        dest = cache_dir() / f"{cid}.webp"
        if dest.is_file():
            status, paths = "downloaded", [str(dest)]
        else:
            try:
                req = urllib.request.Request(
                    CDN_URL.format(cid=cid), headers={"User-Agent": "digimon-card-lookup/1.0"}
                )
                with urllib.request.urlopen(req, timeout=15) as resp:
                    data = resp.read()
                if data[:4] == b"RIFF":  # valid webp
                    dest.write_bytes(data)
                    status, paths = "downloaded", [str(dest)]
            except Exception:
                status = "missing"
    return {"id": cid, "status": status, "paths": paths}


# --- output ----------------------------------------------------------------

def build_manifest(args) -> list[dict]:
    root = repo_root()
    cards, overrides = load_data(root)
    name_idx = index_names(cards)
    arch_pools, alias_map = index_archetypes(root)
    idir = image_dir()

    results = []
    for q in args.queries:
        rtype, ids = resolve_query(q, args.type, cards, name_idx, arch_pools, alias_map)
        entry = {"query": q, "resolved_type": rtype, "cards": []}
        for cid in ids:
            img = resolve_image(cid, idir, args.editions, not args.no_download)
            entry["cards"].append({
                "id": cid,
                "image": img,
                "text": card_text(cid, cards, overrides),
            })
        results.append(entry)
    return results


def print_human(results: list[dict]) -> None:
    all_paths: list[str] = []
    for entry in results:
        q, rtype = entry["query"], entry["resolved_type"]
        n = len(entry["cards"])
        print(f"\n=== query: {q!r}  ->  {rtype}  ({n} card{'s' if n != 1 else ''}) ===")
        if rtype == "unknown" or n == 0:
            print("  (no matches — check spelling, or pass --type id|name|archetype)")
            continue
        for c in entry["cards"]:
            t, img = c["text"], c["image"]
            print(f"\n  [{c['id']}] {t.get('name','')}  "
                  f"({t.get('color','')} | Lv{t.get('level','')} | {t.get('type','')} {t.get('form','')})")
            print(f"    image: {img['status']}  {img['paths'][0] if img['paths'] else '(unavailable)'}")
            for p in img["paths"][1:]:
                print(f"           + {p}")
            if t.get("effect"):
                print(f"    effect    : {t['effect']}")
            if t.get("inherited"):
                print(f"    inherited : {t['inherited']}")
            if t.get("security"):
                print(f"    security  : {t['security']}")
            if t.get("_overrides"):
                print(f"    overrides : {t['_overrides']}")
            all_paths.extend(img["paths"])

    if all_paths:
        print("\n--- IMAGES TO READ (pass these to the Read tool; batch them) ---")
        for p in all_paths:
            print(p)
        unavailable = [c["id"] for e in results for c in e["cards"] if not c["image"]["paths"]]
        if unavailable:
            print(f"\n(!) no image available (local or CDN) for: {', '.join(unavailable)}")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Resolve Digimon card queries to image paths + text.")
    ap.add_argument("queries", nargs="+", help="card IDs, card names, and/or archetype names")
    ap.add_argument("--type", choices=["auto", "id", "name", "archetype"], default="auto",
                    help="force the query type (default: auto-detect)")
    ap.add_argument("--editions", action="store_true",
                    help="for ID queries, also include alt-art editions (_P0/_P1/_P2)")
    ap.add_argument("--no-download", action="store_true",
                    help="do not fetch CDN fallback for images missing locally")
    ap.add_argument("--paths-only", action="store_true",
                    help="print only resolved image paths, one per line")
    ap.add_argument("--json", action="store_true", help="emit the manifest as JSON")
    args = ap.parse_args(argv)

    # Card text contains fullwidth/ideographic chars; force UTF-8 so output
    # never dies on a cp1252 Windows console (no PYTHONIOENCODING needed).
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

    results = build_manifest(args)

    if args.json:
        print(json.dumps(results, indent=2, ensure_ascii=False))
    elif args.paths_only:
        for entry in results:
            for c in entry["cards"]:
                for p in c["image"]["paths"]:
                    print(p)
    else:
        print_human(results)
    return 0


if __name__ == "__main__":
    sys.exit(main())
