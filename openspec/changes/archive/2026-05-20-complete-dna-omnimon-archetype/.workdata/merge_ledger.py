"""Phase E task 9.6 (corrected) — merge DNA Omnimon verdicts into the real
multi-archetype validated_cards_dsl.json, preserving all other archetypes' entries."""
import json, os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
os.chdir(ROOT)
WD = "openspec/changes/complete-dna-omnimon-archetype/.workdata"

# the ORIGINAL file as it was at HEAD (304-card dict, all archetypes)
base = json.load(open(f"{WD}/vcdsl_head.json", encoding="utf-8"))
cards = base["cards"]

enum = json.load(open(f"{WD}/phase-a-enumeration.json", encoding="utf-8"))
pool = sorted(enum["frequency"].keys())

# current open gaps per card (post-cleanup): only BT17-102 and BT23-096
open_by_card = {}
for t in enum["ignored_tests"]:
    open_by_card.setdefault(t["card"], []).append(t["tag"])

card_names = {}
try:
    raw = json.load(open("data/cards.json", encoding="utf-8"))
    for c in (raw if isinstance(raw, list) else raw.get("cards", [])):
        if isinstance(c, dict):
            cid = c.get("card_id") or c.get("id")
            if cid:
                card_names[cid] = c.get("name") or c.get("card_name") or ""
except Exception:
    pass

updated = added = 0
for cid in pool:
    setp = cid.split("-")[0].lower()
    cidl = cid.lower().replace("-", "_")
    gaps = open_by_card.get(cid)
    if gaps:
        status = "PARTIAL"
        gap_kind = ", ".join(sorted(set(gaps)))
        notes = ("DNA Omnimon completion 2026-05-20: core card faithfully implemented; "
                 f"one clause omitted on a verified-open engine gap ({gap_kind}). "
                 "Test left #[ignore]'d with an accurate reason.")
    else:
        status = "IMPLEMENTED"
        gap_kind = None
        notes = ("DNA Omnimon completion 2026-05-20: faithfully implemented in DSL YAML; "
                 "behavioral tests pass; no ignored tests; no raw_rust.")

    if cid in cards:
        e = cards[cid]
        e["status"] = status
        e["gap_kind"] = gap_kind
        e["archetype"] = "DNA Omnimon"
        e["validated_date"] = "2026-05-20"
        e["report"] = "complete-dna-omnimon-archetype"
        e["notes"] = notes
        e["yaml_path"] = f"code/digimon-engine/cards/{setp}/{cid}.yaml"
        e["test_path"] = f"code/digimon-engine/tests/cards_behavioral/{setp}/{cidl}.rs"
        updated += 1
    else:
        cards[cid] = {
            "card_name": card_names.get(cid, ""),
            "validated_date": "2026-05-20",
            "report": "complete-dna-omnimon-archetype",
            "status": status,
            "gap_kind": gap_kind,
            "archetype": "DNA Omnimon",
            "yaml_path": f"code/digimon-engine/cards/{setp}/{cid}.yaml",
            "test_path": f"code/digimon-engine/tests/cards_behavioral/{setp}/{cidl}.rs",
            "test_count": None,
            "patterns": [],
            "notes": notes,
        }
        added += 1

base["last_updated"] = "2026-05-20"
json.dump(base, open("qa/qa-reports/validated_cards_dsl.json", "w", encoding="utf-8"),
          indent=2)

from collections import Counter
dna = [c for c in cards.values() if c.get("archetype") == "DNA Omnimon"]
print(f"merged: total cards {len(cards)} | DNA Omnimon updated {updated} + added {added}")
print("DNA Omnimon verdicts:", dict(Counter(c["status"] for c in dna)))
print("non-DNA entries preserved:", len(cards) - len(dna))
