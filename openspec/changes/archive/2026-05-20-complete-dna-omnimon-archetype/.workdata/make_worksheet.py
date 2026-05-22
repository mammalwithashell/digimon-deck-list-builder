"""Build per-tag classification worksheet for Phase A task 1.3."""
import json, os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
os.chdir(ROOT)
d = json.load(open(
    "openspec/changes/complete-dna-omnimon-archetype/.workdata/phase-a-enumeration.json",
    encoding="utf-8"))
tests = d["ignored_tests"]

bytag = {}
for t in tests:
    key = t["tag"] or ("UNTAGGED::" + t["card"])
    bytag.setdefault(key, []).append(t)

items = []
for key in sorted(bytag):
    ts = bytag[key]
    cards = sorted(set(x["card"] for x in ts))
    reasons = sorted(set(x["reason"] for x in ts if x["reason"]))
    fns = [{"card": x["card"], "fn": x["fn"], "file": x["file"]} for x in ts]
    items.append({
        "key": key,
        "test_count": len(ts),
        "cards": cards,
        "reasons": reasons,
        "tests": fns,
    })

json.dump(items, open(
    "openspec/changes/complete-dna-omnimon-archetype/.workdata/classification-worksheet.json",
    "w", encoding="utf-8"), indent=1)
print("worksheet items:", len(items))
for i, it in enumerate(items):
    print(f"  [{i:2}] {it['key']:42} tests={it['test_count']} cards={','.join(it['cards'])}")
