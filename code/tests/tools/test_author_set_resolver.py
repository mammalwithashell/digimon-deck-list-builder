"""Tests for the release-set resolver (task 1.1 / 1.4)."""

from tools.author_set.set_resolver import (
    normalize_prefix,
    resolve_set,
    resolve_set_from_api_rows,
)

# Minimal synthetic cards dict (keyed by card_id) exercising the prefix edge cases.
SYNTH = {
    "BT1-001": {"card_id": "BT1-001"},
    "BT1-112": {"card_id": "BT1-112"},
    "BT10-001": {"card_id": "BT10-001"},  # must NOT match prefix BT1
    "BT17-001": {"card_id": "BT17-001"},
    "BT17-102": {"card_id": "BT17-102"},
    "EX12-001": {"card_id": "EX12-001"},
    "ST3-01": {"card_id": "ST3-01"},
    "P-001": {"card_id": "P-001"},
}


def test_normalize_prefix():
    assert normalize_prefix("bt17") == "BT17"
    assert normalize_prefix(" ex12 ") == "EX12"


def test_exact_prefix_does_not_bleed_into_longer_set():
    # BT1 must return only BT1-*, never BT10-*/BT17-*.
    assert resolve_set("BT1", SYNTH) == ["BT1-001", "BT1-112"]


def test_case_insensitive():
    assert resolve_set("bt17", SYNTH) == ["BT17-001", "BT17-102"]


def test_two_char_and_promo_prefixes():
    assert resolve_set("EX12", SYNTH) == ["EX12-001"]
    assert resolve_set("ST3", SYNTH) == ["ST3-01"]
    assert resolve_set("P", SYNTH) == ["P-001"]


def test_absent_set_is_empty():
    assert resolve_set("BT99", SYNTH) == []


def test_api_rows_dedupe_alt_art_and_filter_prefix():
    rows = [
        {"id": "BT17-001", "tcgplayer_id": 1},
        {"id": "BT17-001", "tcgplayer_id": 2},  # alt-art dupe row
        {"id": "BT17-002", "tcgplayer_id": 3},
        {"id": "BT170-001", "tcgplayer_id": 4},  # not a real set; must not leak into BT17
    ]
    assert resolve_set_from_api_rows("BT17", rows) == ["BT17-001", "BT17-002"]


def test_resolver_against_real_snapshot_bt17():
    # Integration: the local snapshot has the full settled BT17 set (102 cards).
    ids = resolve_set("BT17")
    assert len(ids) == 102
    assert ids[0] == "BT17-001"
    assert all(c.startswith("BT17-") for c in ids)
