import json

from tools.clause_coverage.book import merge_into_book


def _fake_extract(card_ids, source_desc, **_):
    clauses = [
        {"id": f"{cid}#effect#0", "card_id": cid, "zone": "effect", "label": "Effect",
         "kind": "timing", "timings": [], "keyword": None, "text": f"text {cid}",
         "source": "bundle", "image_path": None}
        for cid in card_ids if cid != "VANILLA-1"
    ]
    return {"generated_at": "t", "source": source_desc, "cards": list(card_ids),
            "clauses": clauses, "denominator": {}}


def _write_book(path, cards, clauses):
    path.write_text(json.dumps({"generated_at": "t", "source": "seed", "cards": cards,
                                "clauses": clauses, "denominator": {}}), encoding="utf-8")


def test_merge_adds_only_missing_cards_and_sorts(tmp_path):
    book = tmp_path / "book.json"
    _write_book(book, ["BT1-001"], [{"id": "BT1-001#effect#0", "label": "Effect", "text": "old"}])
    added = merge_into_book(book, ["BT1-001", "EX10-025"], extract=_fake_extract)
    assert added == ["EX10-025"]
    data = json.loads(book.read_text(encoding="utf-8"))
    assert data["cards"] == ["BT1-001", "EX10-025"]
    assert [c["id"] for c in data["clauses"]] == ["BT1-001#effect#0", "EX10-025#effect#0"]
    assert data["clauses"][0]["text"] == "old", "existing clauses are never re-extracted"


def test_a_zero_clause_card_is_recorded_so_it_is_not_re_extracted(tmp_path):
    book = tmp_path / "book.json"
    _write_book(book, [], [])
    assert merge_into_book(book, ["VANILLA-1"], extract=_fake_extract) == ["VANILLA-1"]
    assert merge_into_book(book, ["VANILLA-1"], extract=_fake_extract) == []


def test_merge_appends_new_cards_and_leaves_the_existing_prefix_untouched(tmp_path):
    # The committed book is two sorted blocks (one per earlier additive merge),
    # NOT globally sorted. A merge must append a new sorted block, never re-sort,
    # or every `book add` would rewrite the whole file in git.
    book = tmp_path / "book.json"
    existing = [{"id": "ST1-01#effect#0", "label": "Effect", "text": "z"},
                {"id": "BT1-001#effect#0", "label": "Effect", "text": "a"}]
    _write_book(book, ["ST1-01", "BT1-001"], existing)
    merge_into_book(book, ["EX9-001", "EX10-025"], extract=_fake_extract)
    data = json.loads(book.read_text(encoding="utf-8"))
    assert data["cards"] == ["ST1-01", "BT1-001", "EX10-025", "EX9-001"]
    assert [c["id"] for c in data["clauses"]][:2] == ["ST1-01#effect#0", "BT1-001#effect#0"]
    assert [c["id"] for c in data["clauses"]][2:] == ["EX10-025#effect#0", "EX9-001#effect#0"]


def test_merge_preserves_the_files_format(tmp_path):
    # Committed book format: indent 2, insertion key order, non-ASCII verbatim,
    # CRLF line endings, no trailing newline.
    book = tmp_path / "book.json"
    seed = {"generated_at": "t", "source": "seed", "cards": ["BT1-001"],
            "clauses": [{"id": "BT1-001#effect#0", "label": "Effect", "text": "【Main】 old"}],
            "denominator": {"total_clauses": 1, "total_cards": 1}}
    book.write_bytes(json.dumps(seed, indent=2, ensure_ascii=False)
                     .replace("\n", "\r\n").encode("utf-8"))
    before = book.read_bytes()
    merge_into_book(book, ["EX10-025"], extract=_fake_extract)
    after = book.read_bytes()
    assert b"\r\n" in after and b"\n" not in after.replace(b"\r\n", b""), "CRLF kept"
    assert not after.endswith(b"\n"), "no trailing newline added"
    assert "【Main】".encode("utf-8") in after, "non-ASCII not escaped"
    assert list(json.loads(after).keys()) == ["generated_at", "source", "cards", "clauses",
                                              "denominator"]
    # Every old line survives verbatim: the diff is additions-only apart from the
    # handful of top-level fields that legitimately change.
    old_lines = before.decode("utf-8").split("\r\n")
    new_lines = after.decode("utf-8").split("\r\n")
    removed = [ln for ln in old_lines if ln not in new_lines]
    assert all(('"source"' in ln or '"total_' in ln or ln.strip() in ('}', '],')
                or ln.rstrip(",").strip() in ('"BT1-001"', '}')) for ln in removed), removed


def test_merge_recomputes_the_denominator_the_way_extract_does(tmp_path):
    book = tmp_path / "book.json"
    _write_book(book, ["BT1-001"], [{"id": "BT1-001#effect#0", "zone": "effect",
                                     "source": "bundle", "label": "Effect", "text": "old"}])
    merge_into_book(book, ["EX10-025"], extract=_fake_extract)
    d = json.loads(book.read_text(encoding="utf-8"))["denominator"]
    assert d["total_cards"] == 2 and d["total_clauses"] == 2
    assert d["by_zone"] == {"effect": 2} and d["by_source"] == {"bundle": 2}
    assert d["image_required_count"] == 0 and d["image_required_clause_ids"] == []
