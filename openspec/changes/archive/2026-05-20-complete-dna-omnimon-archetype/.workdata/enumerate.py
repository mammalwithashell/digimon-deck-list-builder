"""Phase A tasks 1.1 + 1.2 — resolve DNA Omnimon pool + enumerate ignored tests."""
import json, os, re, glob
from collections import Counter

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
os.chdir(ROOT)

d = json.load(open("data/deck_library.json", encoding="utf-8"))
target = None


def walk(o):
    global target
    if isinstance(o, dict):
        for k, v in o.items():
            if str(k).strip().lower() == "dna omnimon":
                target = v
            walk(v)
    elif isinstance(o, list):
        for v in o:
            walk(v)


walk(d)

# 1.1 pool + frequency
decks_with = Counter()
for e in target["decklists"]:
    cards = json.loads(e["decklist"])
    for c in set(cards):
        decks_with[c] += 1
pool = sorted(decks_with.keys())

# 1.2 enumerate ignores — line-anchored to skip doc-comment false positives.
# DOTALL so \\. matches a backslash-newline line-continuation inside the reason string.
IGNORE_RE = re.compile(
    r'^[ \t]*#\[ignore(?:\s*=\s*"((?:[^"\\]|\\.)*)")?\s*\]',
    re.MULTILINE | re.DOTALL,
)
tests = []
yaml_status = {}
for cid in pool:
    setp = cid.split("-")[0].lower()
    cidl = cid.lower().replace("-", "_")
    yp = f"code/digimon-engine/cards/{setp}/{cid}.yaml"
    raw = 0
    if os.path.exists(yp):
        ytxt = open(yp, encoding="utf-8", errors="replace").read()
        raw = ytxt.count("raw_rust")
    yaml_status[cid] = {"has_yaml": os.path.exists(yp), "raw_rust": raw}
    for tp in glob.glob(f"code/digimon-engine/tests/cards_behavioral/{setp}/{cidl}.rs"):
        t = open(tp, encoding="utf-8", errors="replace").read()
        for m in IGNORE_RE.finditer(t):
            reason = (m.group(1) or "")
            reason = reason.replace("\\n", " ").replace("\n", " ")
            reason = re.sub(r"\s+", " ", reason).strip()
            after = t[m.end():m.end() + 400]
            fn = re.search(r"fn\s+([a-z0-9_]+)", after)
            tag = re.search(r"(G-[A-Z0-9-]+)", reason)
            tests.append({
                "card": cid,
                "file": tp.replace("\\", "/"),
                "fn": fn.group(1) if fn else "?",
                "reason": reason,
                "tag": tag.group(1) if tag else None,
            })

out = {
    "pool_size": len(pool),
    "frequency": dict(decks_with),
    "yaml_status": yaml_status,
    "ignored_tests": tests,
}
outp = "openspec/changes/complete-dna-omnimon-archetype/.workdata/phase-a-enumeration.json"
json.dump(out, open(outp, "w", encoding="utf-8"), indent=1)

print("pool size:", len(pool))
print("ignored tests enumerated:", len(tests))
print("with explicit G-tag:", sum(1 for x in tests if x["tag"]))
print("without tag:", sum(1 for x in tests if not x["tag"]))
distinct = sorted(set(x["tag"] for x in tests if x["tag"]))
print("distinct tags:", len(distinct))
for tg in distinct:
    print("  ", tg, "x", sum(1 for x in tests if x["tag"] == tg))
