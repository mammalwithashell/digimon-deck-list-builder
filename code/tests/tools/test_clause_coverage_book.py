import json

import pytest

from tools.clause_coverage import book as book_module
from tools.clause_coverage.book import UnknownCardIds, main, merge_into_book

KNOWN = {"BT1-001", "EX10-025", "EX9-001", "VANILLA-1"}


def _merge(book, ids):
    return merge_into_book(book, ids, extract=_fake_extract, resolve=lambda: KNOWN)


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
    added = _merge(book, ["BT1-001", "EX10-025"])
    assert added == ["EX10-025"]
    data = json.loads(book.read_text(encoding="utf-8"))
    assert data["cards"] == ["BT1-001", "EX10-025"]
    assert [c["id"] for c in data["clauses"]] == ["BT1-001#effect#0", "EX10-025#effect#0"]
    assert data["clauses"][0]["text"] == "old", "existing clauses are never re-extracted"


def test_a_zero_clause_card_is_recorded_so_it_is_not_re_extracted(tmp_path):
    book = tmp_path / "book.json"
    _write_book(book, [], [])
    assert _merge(book, ["VANILLA-1"]) == ["VANILLA-1"]
    assert _merge(book, ["VANILLA-1"]) == []


def test_merge_appends_new_cards_and_leaves_the_existing_prefix_untouched(tmp_path):
    # The committed book is two sorted blocks (one per earlier additive merge),
    # NOT globally sorted. A merge must append a new sorted block, never re-sort,
    # or every `book add` would rewrite the whole file in git.
    book = tmp_path / "book.json"
    existing = [{"id": "ST1-01#effect#0", "label": "Effect", "text": "z"},
                {"id": "BT1-001#effect#0", "label": "Effect", "text": "a"}]
    _write_book(book, ["ST1-01", "BT1-001"], existing)
    _merge(book, ["EX9-001", "EX10-025"])
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
    _merge(book, ["EX10-025"])
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
    _merge(book, ["EX10-025"])
    d = json.loads(book.read_text(encoding="utf-8"))["denominator"]
    assert d["total_cards"] == 2 and d["total_clauses"] == 2
    assert d["by_zone"] == {"effect": 2} and d["by_source"] == {"bundle": 2}
    assert d["image_required_count"] == 0 and d["image_required_clause_ids"] == []


def _seeded(tmp_path):
    book = tmp_path / "book.json"
    _write_book(book, ["BT1-001"], [{"id": "BT1-001#effect#0", "label": "Effect", "text": "old"}])
    return book


def test_an_unknown_id_is_refused_and_the_book_is_untouched(tmp_path):
    book = _seeded(tmp_path)
    before = book.read_bytes()
    with pytest.raises(UnknownCardIds, match="ZZ9-999"):
        _merge(book, ["ZZ9-999"])
    assert book.read_bytes() == before


def test_a_mix_of_known_and_unknown_ids_writes_nothing(tmp_path):
    book = _seeded(tmp_path)
    before = book.read_bytes()
    with pytest.raises(UnknownCardIds) as exc:
        _merge(book, ["EX10-025", "ex10-025", ""])
    assert "ex10-025" in str(exc.value) and "EX10-025" not in str(exc.value).split("(s)")[1]
    assert book.read_bytes() == before, "the known id must not be written either"
    assert [p.name for p in tmp_path.iterdir()] == ["book.json"], "no temp file left behind"


def test_an_already_present_id_never_consults_the_resolver(tmp_path):
    book = _seeded(tmp_path)

    def boom():
        raise AssertionError("resolver must not run when nothing is missing")

    assert merge_into_book(book, ["BT1-001"], extract=_fake_extract, resolve=boom) == []


def test_cli_exits_nonzero_with_the_message_on_stderr(tmp_path, monkeypatch, capsys):
    book = _seeded(tmp_path)
    before = book.read_bytes()
    monkeypatch.setattr(book_module, "known_card_ids", lambda: KNOWN)
    monkeypatch.setattr(book_module, "merge_into_book",
                        lambda p, ids, **kw: merge_into_book(p, ids, extract=_fake_extract,
                                                             resolve=lambda: KNOWN))
    assert main(["add", "--book", str(book), "--card-ids", "ZZ9-999"]) == 2
    assert "ZZ9-999" in capsys.readouterr().err
    assert book.read_bytes() == before
