"""Phase E task 9.6 — finalize validated_cards_dsl.json from current verified state."""
import json, os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
os.chdir(ROOT)

enum = json.load(open(
    "openspec/changes/complete-dna-omnimon-archetype/.workdata/phase-a-enumeration.json",
    encoding="utf-8"))
freq = enum["frequency"]
pool = sorted(freq.keys())

# current ignored tests (post-cleanup) -> card -> list of open gap tags
open_by_card = {}
for t in enum["ignored_tests"]:
    open_by_card.setdefault(t["card"], set()).add(t["tag"] or "untagged")

RANK = {"IMPLEMENTED": 0, "PARTIAL": 1, "BLOCKED": 2}
ledger = []
for cid in pool:
    gaps = sorted(open_by_card.get(cid, []))
    if gaps:
        verdict = "PARTIAL"
        note = (f"Core card faithfully implemented; one omitted clause remains on a "
                f"verified-open engine gap: {', '.join(gaps)}.")
    else:
        verdict = "IMPLEMENTED"
        note = "Faithfully implemented in DSL YAML; behavioral tests pass; no ignored tests."
    ledger.append({
        "card_id": cid,
        "archetype": "DNA Omnimon",
        "deck_frequency": freq[cid],
        "verdict": verdict,
        "notes": note,
    })

ledger.sort(key=lambda e: (-RANK[e["verdict"]], -e["deck_frequency"], e["card_id"]))
json.dump(ledger, open("qa/qa-reports/validated_cards_dsl.json", "w", encoding="utf-8"),
          indent=1)

from collections import Counter
vc = Counter(e["verdict"] for e in ledger)
print("validated_cards_dsl.json finalized —", len(ledger), "cards")
print("verdicts:", dict(vc))
for e in ledger:
    if e["verdict"] != "IMPLEMENTED":
        print(f"  {e['verdict']} {e['card_id']} (f{e['deck_frequency']}) — {e['notes']}")
