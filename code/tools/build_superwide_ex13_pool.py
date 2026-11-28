"""Build the canonical super-wide EX13 training & strictly disjoint held-out pool.

Combines:
  1. Owner-played admissible DCGO recording lists (pinned first, ranked by games played).
  2. Fresh live EX13 DigiLab / deck_library.json tournament lists (ranked by EX13 recency,
     source quality, top-cut, and field size).
  3. Existing curated admissible seeds (e.g. Light Hybrid training list).

Outputs under --out-dir (default data/training_pools/ex13-superwide-20261127):
  - train_snapshot.json          (GeneralistDeckPool schema v1, 3-4 distinct lists/arch)
  - heldout_snapshot.json        (strictly disjoint held-out lists across ALL archetypes)
  - new_archetypes_snapshot.json (subset of train_snapshot for Stage 0b warm-start)
  - decks.json                   (uniform 4 CLI slots per archetype for digimon-engine-cli)
  - heldout_decks.json           (disjoint held-out CLI deck list)
  - pool_report.json             (full audit, per-archetype counts, pinned owner lists, 0 contamination)
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Optional

REPO_ROOT = Path(__file__).resolve().parents[2]
if str(REPO_ROOT / "code") not in sys.path:
    sys.path.insert(0, str(REPO_ROOT / "code"))
if str(REPO_ROOT / "code" / "dcgo-melon-recorder") not in sys.path:
    sys.path.insert(0, str(REPO_ROOT / "code" / "dcgo-melon-recorder"))

# Cards whose printed text permits up to 50 copies in a deck (e.g. Vemmon, Eosmon, etc.)
UP_TO_50_RE = re.compile(r"up to 50 copies", re.IGNORECASE)

SOURCE_PRIORITY = {
    "owner_dcgo": 6,
    "digilab_api": 5,
    "digilab": 4,
    "digimonmeta": 3,
    "egman": 2,
    "dcg_nexus": 2,
    "digimoncard_io": 1,
    "curated": 1,
}

# Prior 14 archetypes already trained in ex13-expanded-lstm384
PRIOR_14_ARCHETYPES = {
    "Chronicle",
    "Chronomon",
    "Examon",
    "Glowing Dawn",
    "Light Hybrid",
    "Medusamon",
    "Royal Knights",
    "SW",
    "ShineGreymon Data Squad",
    "TS Jupitermon",
    "TS Mervamon",
    "Three Musketeers",
    "Toho Braves",
    "Virus Busters",
}


def load_cards_map() -> tuple[dict[str, dict], set[str], set[str]]:
    raw = json.loads((REPO_ROOT / "data" / "cards.json").read_text(encoding="utf-8"))
    cm = {c["card_id"]: c for c in raw} if isinstance(raw, list) else raw
    egg_ids = {
        cid
        for cid, c in cm.items()
        if str(c.get("card_kind", "")).lower() in ("digi-egg", "digiegg", "digitama", "3")
        or str(c.get("type_eng", "")).lower() == "digi-egg"
        or c.get("level") == 2
    }
    up_to_50 = {
        cid
        for cid, c in cm.items()
        if UP_TO_50_RE.search(
            " ".join(
                str(c.get(k) or "")
                for k in ("effect_text", "inherited_text", "effect_description_eng", "inherited_effect_description_eng")
            )
        )
    }
    up_to_50.update({"BT11-061", "BT21-056", "BT6-085", "EX1-065", "EX9-056", "BT14-059"})
    return cm, egg_ids, up_to_50


def canonical_deck_id(card_counts: dict[str, int]) -> str:
    from digimon_gym.agents.gauntlet import stable_deck_id

    expanded = []
    for cid, cnt in sorted(card_counts.items()):
        expanded.extend([cid] * int(cnt))
    return stable_deck_id(expanded)


def expand_card_ids(card_counts: dict[str, int], egg_ids: set[str]) -> list[str]:
    eggs = sorted(cid for cid in card_counts if cid in egg_ids)
    mains = sorted(cid for cid in card_counts if cid not in egg_ids)
    out: list[str] = []
    for cid in eggs + mains:
        out.extend([cid] * card_counts[cid])
    return out


def is_legal_admissible(
    card_counts: dict[str, int],
    *,
    egg_ids: set[str],
    up_to_50: set[str],
    admissible_ids: set[str],
    implemented_ids: set[str],
) -> bool:
    if not card_counts:
        return False
    main_cnt = sum(cnt for cid, cnt in card_counts.items() if cid not in egg_ids)
    egg_cnt = sum(cnt for cid, cnt in card_counts.items() if cid in egg_ids)
    if main_cnt != 50 or egg_cnt > 5:
        return False
    for cid, cnt in card_counts.items():
        if cnt <= 0:
            return False
        max_copies = 50 if cid in up_to_50 else 4
        if cnt > max_copies:
            return False
        if cid not in admissible_ids or cid not in implemented_ids:
            return False
    return True


def deck_l1_distance(a: dict[str, int], b: dict[str, int]) -> int:
    keys = set(a) | set(b)
    return sum(abs(a.get(k, 0) - b.get(k, 0)) for k in keys) // 2


def parse_deck_entry_counts(dl: dict) -> dict[str, int]:
    if isinstance(dl.get("cards"), dict):
        return {str(k).strip().upper(): int(v) for k, v in dl["cards"].items() if int(v) > 0}
    raw = dl.get("decklist")
    if isinstance(raw, str):
        try:
            arr = json.loads(raw)
            if isinstance(arr, list):
                return dict(Counter(str(x).strip().upper() for x in arr))
        except Exception:
            pass
    if isinstance(raw, list):
        return dict(Counter(str(x).strip().upper() for x in raw))
    return {}


def parse_dcgo_txt(path: Path) -> dict[str, int]:
    counts: dict[str, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        m = re.match(r"^\s*(\d+)\s+.+?\s+([A-Z]{1,4}\d*-\d+)\s*$", line)
        if m:
            cnt = int(m.group(1))
            cid = m.group(2).upper()
            counts[cid] = counts.get(cid, 0) + cnt
    return counts


def classify_dcgo_deck(card_counts: dict[str, int], cards_map: dict[str, dict]) -> Optional[str]:
    ids = set(card_counts)
    names = " | ".join(str(cards_map.get(c, {}).get("card_name_eng", "")) for c in ids)
    nl = names.lower()
    traits = " | ".join(str(cards_map.get(c, {}).get("type_eng", "")) for c in ids).lower()
    if any(k in nl for k in ("lobomon", "kendogarurumon", "beowolfmon", "magnagarurumon", "koji minamoto")) or "BT18-097" in ids:
        return "Light Hybrid"
    if any(k in nl for k in ("shinegreymon", "marcus damon", "geogreymon", "rizegreymon")):
        return "ShineGreymon Data Squad"
    if "ravemon" in nl or ("rosemon" in nl and "falcomon" in nl):
        return "Rose Rave"
    if "BT13-007" in ids or "king drasil" in nl or "yggdrasil_7d6" in nl:
        return "Royal Knights"
    if any(k in nl for k in ("shakamon", "sanzomon", "gokuwmon", "sagomon")):
        return "SW"
    if any(k in nl for k in ("examon", "dracomon", "wingdramon", "groundramon")):
        return "Examon"
    if any(k in nl for k in ("jupitermon", "aegiomon", "aegiochusmon")):
        return "TS Jupitermon"
    if any(k in nl for k in ("mervamon", "minervamon")):
        return "TS Mervamon"
    if any(k in nl for k in ("beelstarmon", "gundramon", "magnakidmon")):
        return "Three Musketeers"
    if any(k in nl for k in ("galacticmon", "vemmon", "snatchmon")):
        return "Galacticmon"
    if any(k in nl for k in ("ulforceveedramon", "aeroveedramon")):
        return "UlforceVeedramon"
    if any(k in nl for k in ("gallantmon", "wargrowlmon", "growlmon")):
        return "Gallantmon"
    if any(k in nl for k in ("imperialdramon", "paildramon", "dinobeemon")):
        return "Imperialdramon (UG)"
    if any(k in nl for k in ("jesmon", "saviorhuckmon", "huckmon")):
        return "Jesmon"
    if any(k in nl for k in ("chronomon", "chicchimon")):
        return "Chronomon"
    if any(k in nl for k in ("medusamon", "elizamon", "lamiamon")):
        return "Medusamon"
    if any(c in ids for c in ("EX12-001", "EX12-069", "EX12-073", "EX13-031", "EX13-028", "EX13-064", "BT24-039")) and (
        "EX12-069" in ids or "EX12-073" in ids or "EX12-032" in ids or "virus busters" in traits
    ):
        return "Virus Busters"
    if "adventure" in traits and any(k in nl for k in ("omnimon", "wargreymon", "metalgarurumon")):
        return "Adventure Omni"
    if "machinedramon" in nl or "chaosdramon" in nl:
        return "Metal Empire"
    if any(k in nl for k in ("alphamon", "ouryumon", "dorumon")):
        return "Chronicle"
    if any(k in nl for k in ("amaterasumon", "sanmyojin", "toho", "gammamon", "siriusmon")):
        return "Toho Braves"
    return None


def scan_owner_dcgo_decks(
    cards_map: dict[str, dict],
    egg_ids: set[str],
    up_to_50: set[str],
    admissible_ids: set[str],
    implemented_ids: set[str],
) -> tuple[dict[str, list[dict]], dict[str, list[dict]]]:
    import convert_raw

    import os

    raw_dirs = os.environ.get("DCGO_RECORDING_DIRS", "")
    dirs = [Path(p.strip()) for p in raw_dirs.split(os.pathsep) if p.strip()]
    seen_raw: dict[str, Path] = {}
    for d in dirs:
        if d.exists():
            for f in sorted(d.glob("raw-*.jsonl")):
                if f.name not in seen_raw or f.stat().st_size > seen_raw[f.name].stat().st_size:
                    seen_raw[f.name] = f

    owner_map: dict[str, dict] = {}
    opp_map: dict[str, dict] = {}

    for fname, fpath in sorted(seen_raw.items()):
        rows = [json.loads(l) for l in fpath.open(encoding="utf-8", errors="replace") if l.strip()]
        games, cur = [], None
        for r in rows:
            if r.get("method") == "TurnStateMachine.Init":
                cur = []
                games.append(cur)
            if cur is not None:
                cur.append(r)
        for gi, grows in enumerate(games, 1):
            if not any(r.get("method", "").startswith("Snapshot.") for r in grows):
                continue
            try:
                out, _ = convert_raw.convert(grows, cards_map, allow_truncated=True)
            except BaseException:
                continue
            init = next((r for r in out if r.get("type") == "initial_state"), None)
            if not init:
                continue
            for side, target in (("my", owner_map), ("opp", opp_map)):
                pz = init[side]
                cards = (
                    list(pz.get("library_order", []))
                    + list(pz.get("security_order", []))
                    + list(pz.get("initial_hand", []))
                    + list(pz.get("digitama_library_order", []))
                )
                counts = dict(Counter(cards))
                if not is_legal_admissible(
                    counts,
                    egg_ids=egg_ids,
                    up_to_50=up_to_50,
                    admissible_ids=admissible_ids,
                    implemented_ids=implemented_ids,
                ):
                    continue
                arch = classify_dcgo_deck(counts, cards_map)
                if not arch:
                    continue
                did = canonical_deck_id(counts)
                rec = target.setdefault(
                    did,
                    {
                        "archetype": arch,
                        "card_counts": counts,
                        "deck_id": did,
                        "source": "owner_dcgo" if side == "my" else "opp_dcgo",
                        "games": 0,
                        "sources": [],
                    },
                )
                rec["games"] += 1
                rec["sources"].append(f"{fname}#g{gi}")

    # Also ensure the Virus Busters owner list from ex13-expanded-20261126 is captured even if
    # card text classification differed
    prev_snap = REPO_ROOT / "data" / "training_pools" / "ex13-expanded-20261126" / "train_snapshot.json"
    if prev_snap.exists():
        prev = json.loads(prev_snap.read_text(encoding="utf-8"))
        for d in prev.get("decks", []):
            if d.get("source") in ("owner_dcgo", "dcgo_owner_recording"):
                counts = {k: int(v) for k, v in d["card_counts"].items()}
                if is_legal_admissible(
                    counts,
                    egg_ids=egg_ids,
                    up_to_50=up_to_50,
                    admissible_ids=admissible_ids,
                    implemented_ids=implemented_ids,
                ):
                    did = canonical_deck_id(counts)
                    if did not in owner_map:
                        owner_map[did] = {
                            "archetype": d["archetype"],
                            "card_counts": counts,
                            "deck_id": did,
                            "source": "owner_dcgo",
                            "games": 2,
                            "sources": [d.get("source_deck_id", "expanded_pool")],
                        }

    owner_by_arch: dict[str, list[dict]] = defaultdict(list)
    for rec in owner_map.values():
        owner_by_arch[rec["archetype"]].append(rec)
    for arch in owner_by_arch:
        owner_by_arch[arch].sort(key=lambda r: (-r["games"], r["sources"][-1], r["deck_id"]))

    opp_by_arch: dict[str, list[dict]] = defaultdict(list)
    for rec in opp_map.values():
        opp_by_arch[rec["archetype"]].append(rec)
    for arch in opp_by_arch:
        opp_by_arch[arch].sort(key=lambda r: (-r["games"], r["sources"][-1], r["deck_id"]))

    return owner_by_arch, opp_by_arch


# Canonical super-wide EX13 archetype definitions:
# (canonical_name, library_archetype_names, optional_card_filter_fn, max_owner_pinned_in_train)
def _is_shinegreymon(counts: dict[str, int], cm: dict[str, dict]) -> bool:
    names = " ".join(str(cm.get(c, {}).get("card_name_eng", "")).lower() for c in counts)
    return "shinegreymon" in names or "marcus damon" in names


def _is_adventure_omni(counts: dict[str, int], cm: dict[str, dict]) -> bool:
    names = " ".join(str(cm.get(c, {}).get("card_name_eng", "")).lower() for c in counts)
    return "omnimon" in names or "tai kamiya" in names or "wargreymon" in names


ARCHETYPE_SPECS: list[tuple[str, list[str], Optional[Callable[[dict[str, int], dict[str, dict]], bool]], int]] = [
    ("Examon", ["Examon"], None, 1),
    ("TS Jupitermon", ["TS Jupitermon", "Jupitermon"], None, 2),
    ("Glowing Dawn", ["Glowing Dawn"], None, 1),
    ("Three Musketeers", ["Three Musketeers", "Beelstarmon (3M)"], None, 1),
    ("TS Mervamon", ["TS Mervamon"], None, 1),
    ("Chronicle", ["Chronicle"], None, 1),
    ("Toho Braves", ["Toho Braves"], None, 2),
    ("Chronomon", ["Chronomon", "TS Chronomon"], None, 1),
    ("Medusamon", ["Medusamon"], None, 1),
    ("Royal Knights", ["Royal Knights"], None, 2),
    ("ShineGreymon Data Squad", ["DATA SQUAD ShineGreymon", "Data Squad", "ShineGreymon"], _is_shinegreymon, 3),
    ("Virus Busters", ["Virus Busters"], None, 2),
    ("SW", ["Saiyu Warriors"], None, 3),
    ("Light Hybrid", ["Light Hybrid", "Light Hybrid Toolbox", "Hybrid Beowolf"], None, 3),
    ("Rose Rave", ["DATA SQUAD Ravemon", "DATA SQUAD Rosemon", "DATS Ravemon"], None, 2),
    ("Adventure Omni", ["Adventure Omni", "Adventure"], _is_adventure_omni, 2),
    ("Galacticmon", ["Galacticmon", "Vemmon", "Galactic"], None, 2),
    ("UlforceVeedramon", ["UlforceVeedramon", "Ulforce"], None, 2),
    ("Gallantmon", ["Gallantmon", "Dukemon"], None, 2),
    ("Imperialdramon (UG)", ["BG Imperial", "Blue Green Imperialdramon"], None, 2),
    ("Metal Empire", ["Metal Empire"], None, 2),
    ("Jesmon", ["Jesmon", "BT23 Jesmon", "Jesmon GX (Gankoomon)"], None, 2),
    ("TS Gear Forest", ["TS Gear Forest"], None, 1),
    ("TS Vulcanusmon", ["TS Vulcanusmon", "Vulcanusmon"], None, 1),
    ("Dynasmon", ["Dynasmon"], None, 1),
    ("TS Plutomon", ["TS Plutomon", "Plutomon"], None, 1),
]


def parse_date_sort_key(dstr: Optional[str]) -> str:
    if not dstr:
        return "0000-00-00"
    s = str(dstr).strip()
    if re.match(r"^\d{4}-\d{2}-\d{2}", s):
        return s[:10]
    m = re.match(r"^(\d{1,2})/(\d{1,2})/(\d{4})$", s)
    if m:
        return f"{int(m.group(3)):04d}-{int(m.group(1)):02d}-{int(m.group(2)):02d}"
    return "0000-00-00"


def pick_diverse(candidates: list[dict], target: int, already_picked: list[dict], min_diff: int = 2) -> list[dict]:
    chosen: list[dict] = []
    seen_ids = {d["deck_id"] for d in already_picked}
    # Pass 1: enforce >= min_diff card swaps from already chosen lists
    for cand in candidates:
        if len(chosen) >= target:
            break
        if cand["deck_id"] in seen_ids:
            continue
        if all(deck_l1_distance(cand["card_counts"], x["card_counts"]) >= min_diff for x in already_picked + chosen):
            chosen.append(cand)
            seen_ids.add(cand["deck_id"])
    # Pass 2: relax min_diff to >= 1 if still short
    if len(chosen) < target:
        for cand in candidates:
            if len(chosen) >= target:
                break
            if cand["deck_id"] in seen_ids:
                continue
            chosen.append(cand)
            seen_ids.add(cand["deck_id"])
    return chosen


def build_snapshot(decks: list[dict], generated_at: str) -> dict:
    from digimon_gym.agents.gauntlet import _snapshot_hash

    decks_sorted = sorted(decks, key=lambda d: (d["archetype"], d["deck_id"]))
    archs = sorted({d["archetype"] for d in decks_sorted})
    content = {
        "schema_version": 1,
        "sampling_policy": "uniform_archetype_then_deck",
        "eligible_archetypes": archs,
        "decks": decks_sorted,
    }
    return {
        **content,
        "generated_at": generated_at,
        "snapshot_hash": _snapshot_hash(content),
    }


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--out-dir", default="data/training_pools/ex13-superwide-20261127")
    p.add_argument("--train-per-arch", type=int, default=4)
    p.add_argument("--heldout-per-arch", type=int, default=2)
    p.add_argument("--slots-per-arch", type=int, default=4)
    args = p.parse_args()

    import digimon_engine

    implemented_ids = set(digimon_engine.load_implemented_card_ids())
    adm_doc = json.loads((REPO_ROOT / "data" / "card_admissibility.json").read_text(encoding="utf-8"))
    admissible_ids = {cid for cid, v in adm_doc["cards"].items() if v.get("admissible")}
    cards_map, egg_ids, up_to_50 = load_cards_map()

    lib = json.loads((REPO_ROOT / "data" / "deck_library.json").read_text(encoding="utf-8"))
    lib_archs = lib.get("archetypes", {})

    owner_by_arch, opp_by_arch = scan_owner_dcgo_decks(
        cards_map, egg_ids, up_to_50, admissible_ids, implemented_ids
    )

    # Curated admissible Light Hybrid seed from qa/archetype-qa/dsl/light-hybrid-training/dcgo-deck.txt
    curated_by_arch: dict[str, list[dict]] = defaultdict(list)
    lh_txt = REPO_ROOT / "qa" / "archetype-qa" / "dsl" / "light-hybrid-training" / "dcgo-deck.txt"
    if lh_txt.exists():
        c = parse_dcgo_txt(lh_txt)
        if is_legal_admissible(
            c, egg_ids=egg_ids, up_to_50=up_to_50, admissible_ids=admissible_ids, implemented_ids=implemented_ids
        ):
            curated_by_arch["Light Hybrid"].append(
                {
                    "archetype": "Light Hybrid",
                    "card_counts": c,
                    "deck_id": canonical_deck_id(c),
                    "source": "curated",
                    "source_deck_id": "qa_light_hybrid_training",
                    "source_url": str(lh_txt.relative_to(REPO_ROOT)),
                }
            )

    train_decks: list[dict] = []
    heldout_decks: list[dict] = []
    cli_train_slots: list[dict] = []
    cli_heldout_slots: list[dict] = []
    report_archs: dict[str, dict] = {}
    pinned_owner_report: list[dict] = []

    for canon_arch, lib_names, filter_fn, max_owner in ARCHETYPE_SPECS:
        # 1. Gather library candidates ranked by EX13 format -> source priority -> top_cut -> date -> players
        lib_cands: list[dict] = []
        seen_lib_ids: set[str] = set()
        for lname in lib_names:
            adata = lib_archs.get(lname)
            if not adata:
                continue
            for idx, dl in enumerate(adata.get("decklists", [])):
                counts = parse_deck_entry_counts(dl)
                if not is_legal_admissible(
                    counts,
                    egg_ids=egg_ids,
                    up_to_50=up_to_50,
                    admissible_ids=admissible_ids,
                    implemented_ids=implemented_ids,
                ):
                    continue
                if filter_fn is not None and not filter_fn(counts, cards_map):
                    continue
                did = canonical_deck_id(counts)
                if did in seen_lib_ids:
                    continue
                seen_lib_ids.add(did)
                fmt = str(dl.get("format") or dl.get("format_tag") or "").upper()
                src = str(dl.get("source") or "unknown")
                is_ex13 = 1 if fmt == "EX13" else 0
                is_tc = 1 if dl.get("is_top_cut") else 0
                dt = parse_date_sort_key(dl.get("event_date"))
                players = int(dl.get("event_players") or 0)
                lib_cands.append(
                    {
                        "archetype": canon_arch,
                        "card_counts": counts,
                        "deck_id": did,
                        "source": src,
                        "source_deck_id": str(dl.get("deck_id") or f"{lname}_{idx}"),
                        "source_url": str(dl.get("source_url") or ""),
                        "_rank": (is_ex13, SOURCE_PRIORITY.get(src, 0), is_tc, dt, players, did),
                    }
                )
        lib_cands.sort(key=lambda x: x["_rank"], reverse=True)

        # 2. Gather owner DCGO lists + opponent DCGO lists + curated lists
        owners = list(owner_by_arch.get(canon_arch, []))
        opps = list(opp_by_arch.get(canon_arch, []))
        curated = list(curated_by_arch.get(canon_arch, []))

        # Pin top diverse owner lists into train (up to max_owner)
        picked_train: list[dict] = []
        if owners:
            owner_train = pick_diverse(owners, min(max_owner, args.train_per_arch), [], min_diff=1)
            picked_train.extend(owner_train)
            for o in owner_train:
                pinned_owner_report.append(
                    {
                        "archetype": canon_arch,
                        "deck_id": o["deck_id"],
                        "games_played_by_owner": o["games"],
                        "latest_source": o["sources"][-1],
                    }
                )

        # Fill remaining train slots from library -> curated -> opp_dcgo -> remaining owner
        fallback_pool = lib_cands + curated + opps + owners
        need = args.train_per_arch - len(picked_train)
        if need > 0:
            picked_train.extend(pick_diverse(fallback_pool, need, picked_train, min_diff=2))

        # Special safeguard for archetypes with very few total unique admissible lists (e.g. Light Hybrid):
        # ensure at least 1 strictly disjoint heldout list if >= 2 unique lists exist!
        all_unique = []
        seen_u = set()
        for item in owners + lib_cands + curated + opps:
            if item["deck_id"] not in seen_u:
                all_unique.append(item)
                seen_u.add(item["deck_id"])

        train_ids = {d["deck_id"] for d in picked_train}
        remaining_for_heldout = [d for d in lib_cands + owners + curated + opps if d["deck_id"] not in train_ids]
        picked_heldout = pick_diverse(remaining_for_heldout, args.heldout_per_arch, picked_train, min_diff=2)

        # If heldout is empty but we have >= 2 unique lists in train, move the last train list to heldout
        if not picked_heldout and len(picked_train) >= 2:
            picked_heldout = [picked_train.pop()]

        def _clean(rec: dict) -> dict:
            counts = dict(sorted(rec["card_counts"].items()))
            return {
                "archetype": canon_arch,
                "card_counts": counts,
                "card_ids": expand_card_ids(counts, egg_ids),
                "deck_id": rec["deck_id"],
                "source": rec.get("source", "unknown"),
                "source_deck_id": rec.get("source_deck_id") or (rec.get("sources", ["dcgo"])[-1]),
                "source_url": rec.get("source_url", ""),
            }

        clean_train = [_clean(r) for r in picked_train]
        clean_heldout = [_clean(r) for r in picked_heldout]
        assert clean_train, f"No admissible train lists found for {canon_arch}!"

        train_decks.extend(clean_train)
        heldout_decks.extend(clean_heldout)

        # Uniform CLI train slots (repeat round-robin only if < slots_per_arch)
        for slot_i in range(args.slots_per_arch):
            d = clean_train[slot_i % len(clean_train)]
            cli_train_slots.append({"name": f"{canon_arch}:{d['deck_id']}", "cards": d["card_ids"]})

        for d in clean_heldout:
            cli_heldout_slots.append({"name": f"{canon_arch}:{d['deck_id']}", "cards": d["card_ids"]})

        report_archs[canon_arch] = {
            "train_distinct": len(clean_train),
            "heldout_distinct": len(clean_heldout),
            "owner_pinned": sum(1 for d in clean_train if d["source"] == "owner_dcgo"),
            "ex13_lib_admissible": sum(1 for x in lib_cands if x["_rank"][0] == 1),
            "total_lib_admissible": len(lib_cands),
            "sources_train": dict(Counter(d["source"] for d in clean_train)),
            "is_new_archetype": canon_arch not in PRIOR_14_ARCHETYPES,
        }

    # Verify zero holdout contamination
    train_id_set = {d["deck_id"] for d in train_decks}
    heldout_id_set = {d["deck_id"] for d in heldout_decks}
    overlap = train_id_set & heldout_id_set
    assert not overlap, f"Holdout contamination detected: {overlap}"

    now_iso = datetime.now(timezone.utc).isoformat()
    train_snap = build_snapshot(train_decks, now_iso)
    heldout_snap = build_snapshot(heldout_decks, now_iso)
    new_or_lh = [d for d in train_decks if (d["archetype"] not in PRIOR_14_ARCHETYPES) or d["archetype"] == "Light Hybrid"]
    new_snap = build_snapshot(new_or_lh, now_iso)

    out_dir = REPO_ROOT / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "train_snapshot.json").write_text(json.dumps(train_snap, indent=2) + "\n", encoding="utf-8")
    (out_dir / "heldout_snapshot.json").write_text(json.dumps(heldout_snap, indent=2) + "\n", encoding="utf-8")
    (out_dir / "new_archetypes_snapshot.json").write_text(json.dumps(new_snap, indent=2) + "\n", encoding="utf-8")
    (out_dir / "decks.json").write_text(json.dumps(cli_train_slots, indent=2) + "\n", encoding="utf-8")
    (out_dir / "heldout_decks.json").write_text(json.dumps(cli_heldout_slots, indent=2) + "\n", encoding="utf-8")

    # End-to-end verification via GeneralistDeckPool.from_snapshot
    from digimon_gym.agents.gauntlet import GeneralistDeckPool

    for snap_name in ("train_snapshot.json", "heldout_snapshot.json", "new_archetypes_snapshot.json"):
        GeneralistDeckPool.from_snapshot(out_dir / snap_name, implemented_card_ids=implemented_ids)

    report = {
        "schema": "ex13-superwide-pool-v1",
        "generated_utc": now_iso,
        "admissible_cards_total": len(admissible_ids),
        "implemented_cards_total": len(implemented_ids),
        "archetypes_count": len(ARCHETYPE_SPECS),
        "distinct_train_lists": len(train_decks),
        "cli_train_slots": len(cli_train_slots),
        "slots_per_archetype": args.slots_per_arch,
        "heldout_lists": len(heldout_decks),
        "heldout_archetypes": len(heldout_snap["eligible_archetypes"]),
        "holdout_contamination": len(overlap),
        "new_archetypes_subset_count": len(new_snap["eligible_archetypes"]),
        "pinned_owner_dcgo_lists": pinned_owner_report,
        "by_archetype": report_archs,
    }
    (out_dir / "pool_report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    print(f"Built super-wide EX13 pool at {out_dir}")
    print(
        f"  Archetypes: {report['archetypes_count']} | Train distinct: {report['distinct_train_lists']} "
        f"({report['cli_train_slots']} CLI slots) | Held-out: {report['heldout_lists']} across "
        f"{report['heldout_archetypes']} archetypes | Contamination: {report['holdout_contamination']}"
    )
    print(f"  Pinned owner DCGO lists: {len(pinned_owner_report)}")
    for arch, st in report_archs.items():
        tag = "NEW" if st["is_new_archetype"] else "   "
        print(
            f"  [{tag}] {arch:25s} | train={st['train_distinct']} heldout={st['heldout_distinct']} "
            f"| owner={st['owner_pinned']} | EX13_lib={st['ex13_lib_admissible']:3d} | {st['sources_train']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
