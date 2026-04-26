"""Tests for tools.build_card_meta CLI."""
from __future__ import annotations

from pathlib import Path

import pytest

from tools.build_card_meta import build_one, set_id_from_card_id, write_card_meta


def test_set_id_from_card_id_standard():
    assert set_id_from_card_id("BT17-007") == "bt17"
    assert set_id_from_card_id("ST2-13") == "st2"
    assert set_id_from_card_id("AD1-005") == "ad1"


def test_set_id_from_card_id_promo_falls_back_to_misc():
    # Promo-style card_ids without a hyphen, or with non-set prefixes
    assert set_id_from_card_id("PROMO123") == "_misc"


def test_build_one_writes_file_with_lf_newlines(tmp_path: Path):
    # write_card_meta(card_id, root) must produce <root>/<set>/<card_id>.md
    out = write_card_meta("BT17-007", tmp_path)
    assert out == tmp_path / "bt17" / "BT17-007.md"
    assert out.exists()
    raw = out.read_bytes()
    # Reject CRLF — Windows write_text default would corrupt diffability.
    assert b"\r\n" not in raw
    # H1 starts the file
    assert raw.decode("utf-8").startswith("# BT17-007 — Agumon")


def test_build_one_returns_parse_stats():
    # build_one returns (card_id, n_parsed, n_unparsed) without writing
    cid, n_parsed, n_unparsed = build_one("BT17-007")
    assert cid == "BT17-007"
    assert n_parsed == 1
    assert n_unparsed == 0
