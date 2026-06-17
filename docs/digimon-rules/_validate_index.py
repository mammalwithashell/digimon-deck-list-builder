import json, re, sys
from pathlib import Path

base = Path(__file__).resolve().parent
idx = json.loads((base / "rules-index.json").read_text(encoding="utf-8"))
assert idx["source"]["pdf"] == "general_rule.pdf"

for grp in ("keywords", "topics"):
    for key, v in idx[grp].items():
        assert v["names"] and all(isinstance(n, str) and n for n in v["names"]), key
        assert re.fullmatch(r"\d+(-\d+)?", v["pages"]), (grp, key, v["pages"])
        assert v["pdf"] in idx["pdfs"], (grp, key, v["pdf"])

# every keyword in the index appears as a row in the semantics table
table = (base / "keyword-semantics.md").read_text(encoding="utf-8").lower()
missing = [k for k, v in idx["keywords"].items() if v["names"][0].lower() not in table]
assert not missing, f"keywords missing from semantics table: {missing}"

# every section a keyword cites resolves to a known section number
for key, v in idx["keywords"].items():
    sec = v["section"].split("-")[0]
    assert sec in idx["sections"], (key, v["section"])

print(f"rules-index.json OK ({len(idx['keywords'])} keywords, {len(idx['topics'])} topics, {len(idx['sections'])} sections)")
