"""code/tools/import_official_qa.py imports only rulings that trace to Bandai's Q&A pages,
and data/card_official_qa.json holds nothing else."""
import json
import os
import re
import sys

import pytest

_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(_ROOT, "code", "tools"))
import import_official_qa as iq  # noqa: E402

GOOD = {"qno": "Q2668", "date": "2024-03-28", "question": "Does it activate?",
        "answer": "Yes, it activates.", "related": []}


def test_manifest_must_name_the_official_rule_pages():
    iq.check_manifest({"qa": {"url": "https://world.digimoncard.com/rule/?card_no=<card_no>"}})
    for url in ("https://digimoncard.io/api/qa", "http://world.digimoncard.com/rule/?card_no=x",
                "https://world.digimoncard.com.evil.example/rule/", "https://world.digimoncard.com/cardlist/"):
        with pytest.raises(iq.ProvenanceError):
            iq.check_manifest({"qa": {"url": url}})


def test_official_shaped_entries_are_imported_with_their_page():
    out, report = iq.select_rulings({"BT16-082": [GOOD]})
    assert out["BT16-082"] == [dict(GOOD, source_url="https://world.digimoncard.com/rule/?card_no=BT16-082")]
    assert not report["rejected"]


@pytest.mark.parametrize("bad", [
    dict(GOOD, qno="A1"), dict(GOOD, qno="aegis-1"), dict(GOOD, date="Mar. 28, 2024"),
    dict(GOOD, answer=" "), dict(GOOD, related="BT1-001"), dict(GOOD, related=["not a card"]),
    {k: v for k, v in GOOD.items() if k != "question"},
])
def test_entries_not_shaped_like_an_official_ruling_are_rejected(bad):
    out, report = iq.select_rulings({"BT16-082": [bad]})
    assert not out and report["rejected"]


def test_crawler_annotations_are_dropped_not_imported():
    out, report = iq.select_rulings({"BT16-082": [dict(GOOD, interpretation="engine should X", status="proven")]})
    assert set(out["BT16-082"][0]) == set(iq.FIELDS) | {"source_url"}
    assert report["dropped_fields"] == {"interpretation": 1, "status": 1}


def test_a_q_number_with_two_texts_is_rejected_everywhere():
    out, report = iq.select_rulings({"BT1-001": [GOOD], "BT1-002": [dict(GOOD, answer="No.")]})
    assert not out and report["inconsistent_qnos"] == ["Q2668"]


def test_bundle_section_is_replaced_or_inserted_and_idempotent():
    rs = [dict(GOOD, answer="Yes.\nSecond line.", related=["BT12-089"])]
    with_section = "# X\n\n## Official Q&A\n- Yes, you can.\n\n## Image\n- x.webp\n"
    new = iq.rewrite_bundle_text(with_section, "BT16-082", rs)
    assert "- Yes, you can." not in new
    assert "**Q2668** (2024-03-28) Q: Does it activate? — A: Yes. Second line. _(related: BT12-089)_" in new
    assert new.index("## Official Q&A") < new.index("## Image")
    assert iq.rewrite_bundle_text(new, "BT16-082", rs) == new
    inserted = iq.rewrite_bundle_text("# X\n\n## Image\n- x.webp\n", "BT16-082", rs)
    assert inserted.index("## Official Q&A") < inserted.index("## Image")


def test_committed_sidecar_holds_only_official_rulings():
    path = os.path.join(_ROOT, "data", "card_official_qa.json")
    if not os.path.exists(path):
        pytest.skip("data/card_official_qa.json not generated")
    data = json.load(open(path, encoding="utf-8"))
    assert "world.digimoncard.com/rule/" in data["_provenance"]["source"]
    n = 0
    for cid, rs in data["cards"].items():
        for r in rs:
            n += 1
            assert set(r) == set(iq.FIELDS) | {"source_url"}, (cid, r.get("qno"))
            assert re.match(r"^Q\d+$", r["qno"]), (cid, r["qno"])
            assert r["source_url"] == f"https://world.digimoncard.com/rule/?card_no={cid}"
    assert n == data["count"]
