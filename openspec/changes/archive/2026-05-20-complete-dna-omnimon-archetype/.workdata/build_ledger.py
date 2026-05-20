"""Phase A task 1.7 — build validated_cards_dsl.json verdict ledger for DNA Omnimon."""
import json, os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
os.chdir(ROOT)

enum = json.load(open(
    "openspec/changes/complete-dna-omnimon-archetype/.workdata/phase-a-enumeration.json",
    encoding="utf-8"))
clf = json.load(open(
    "openspec/changes/complete-dna-omnimon-archetype/.workdata/classification.json",
    encoding="utf-8"))

freq = enum["frequency"]
ystat = enum["yaml_status"]
pool = sorted(freq.keys())

# invert classification: card -> list of (class, tag, size, note)
card_items = {}
for it in clf["items"]:
    for c in it["cards"]:
        card_items.setdefault(c, []).append(it)

RANK = {"IMPLEMENTED": 0, "PARTIAL": 1, "BLOCKED": 2}
ledger = []
for cid in pool:
    items = card_items.get(cid, [])
    has_yaml = ystat[cid]["has_yaml"]
    raw = ystat[cid]["raw_rust"]
    classes = [i["class"] for i in items]
    open_gaps = sorted({i["tag"] for i in items if i["class"] == "OPEN-SUBSTRATE"})
    auth_gaps = sorted({i["tag"] for i in items if i["class"] == "AUTHORING-ONLY"})
    stale_gaps = sorted({i["tag"] for i in items if i["class"] == "STALE"})
    xtrack = [i["tag"] for i in items if i["class"] == "CROSS-TRACK"]

    if not has_yaml:
        verdict = "BLOCKED"
        note = "No production YAML authored."
        if cid == "BT22-084":
            note += " Example YAML exists in cards/_examples/; substrate present (only G-OUTER-OPTIONAL-NOT-INSTALLED open)."
    elif open_gaps:
        verdict = "PARTIAL"
        note = f"Omitted clause(s) blocked on substrate gap(s): {', '.join(open_gaps)}."
        if auth_gaps:
            note += f" Also authoring-only: {', '.join(auth_gaps)}."
    elif auth_gaps:
        verdict = "PARTIAL"
        note = f"No substrate gap — card clause/test authoring outstanding: {', '.join(auth_gaps)}."
    elif xtrack:
        verdict = "PARTIAL"
        note = "Blocked by a test-fixture defect (make_lv4_digimon empty evo_costs), not an engine gap."
    elif stale_gaps:
        verdict = "IMPLEMENTED"
        note = (f"Substrate verified present; {len(items)} stale #[ignore] marker(s) "
                f"pending re-enable: {', '.join(stale_gaps)}.")
    else:
        verdict = "IMPLEMENTED"
        note = "Production YAML + behavioral tests present; no ignored tests."

    if raw and verdict != "BLOCKED":
        note += f" ({raw} raw_rust escape(s) to review.)"

    ledger.append({
        "card_id": cid,
        "archetype": "DNA Omnimon",
        "deck_frequency": freq[cid],
        "verdict": verdict,
        "notes": note,
    })

ledger.sort(key=lambda e: (-RANK[e["verdict"]], -e["deck_frequency"], e["card_id"]))

os.makedirs("qa/qa-reports", exist_ok=True)
outp = "qa/qa-reports/validated_cards_dsl.json"
json.dump(ledger, open(outp, "w", encoding="utf-8"), indent=1)

from collections import Counter
vc = Counter(e["verdict"] for e in ledger)
print("wrote", outp, "-", len(ledger), "cards")
print("verdicts:", dict(vc))
print()
for e in ledger:
    if e["verdict"] != "IMPLEMENTED":
        print(f"  {e['verdict']:11} {e['card_id']:10} (f{e['deck_frequency']:>2}) {e['notes'][:95]}")
