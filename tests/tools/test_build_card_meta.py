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


def test_check_mode_passes_when_tree_matches(tmp_path: Path, monkeypatch):
    from tools import build_card_meta as m
    # Point CARD_META_ROOT at a fresh tempdir, populate, then check.
    monkeypatch.setattr(m, "CARD_META_ROOT", tmp_path)
    m.write_card_meta("BT17-007", tmp_path)
    m.write_card_meta("ST2-13", tmp_path)
    rc = m.cmd_check(card_ids=["BT17-007", "ST2-13"])
    assert rc == 0


def test_check_mode_fails_on_mismatch(tmp_path: Path, monkeypatch, capsys):
    from tools import build_card_meta as m
    monkeypatch.setattr(m, "CARD_META_ROOT", tmp_path)
    m.write_card_meta("BT17-007", tmp_path)
    # Corrupt the file
    out = tmp_path / "bt17" / "BT17-007.md"
    out.write_text("STALE\n", encoding="utf-8", newline="\n")
    rc = m.cmd_check(card_ids=["BT17-007"])
    assert rc == 1
    captured = capsys.readouterr()
    assert "BT17-007" in captured.err


def test_check_mode_fails_on_missing_file(tmp_path: Path, monkeypatch, capsys):
    from tools import build_card_meta as m
    monkeypatch.setattr(m, "CARD_META_ROOT", tmp_path)
    rc = m.cmd_check(card_ids=["BT17-007"])
    assert rc == 1
    captured = capsys.readouterr()
    assert "missing" in captured.err.lower() or "BT17-007" in captured.err
