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

# the committed text copy (cr-<ver>.txt), when present, must carry the same version stamp
copies = sorted(base.glob("cr-*.txt"))
if copies:
    head = copies[-1].read_text(encoding="utf-8", errors="replace")[:600]
    ver = re.search(r"Ver\.\s*(\d+\.\d+)", head)
    date = re.search(r"Last Updated:\s*(\d{4}/\d{2}/\d{2})", head)
    assert ver and ver.group(1) in idx["source"]["version"], (copies[-1].name, idx["source"]["version"])
    assert date and date.group(1) == idx["source"]["last_updated"], (copies[-1].name, idx["source"]["last_updated"])

print(f"rules-index.json OK ({len(idx['keywords'])} keywords, {len(idx['topics'])} topics, {len(idx['sections'])} sections)")
