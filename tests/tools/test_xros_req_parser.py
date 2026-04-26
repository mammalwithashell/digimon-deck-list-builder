"""Tests for tools.xros_req_parser."""
from __future__ import annotations

import pytest

from tools.xros_req_parser import (
    ParsedAltPath,
    XrosReqParseResult,
    parse,
)


def test_empty_returns_empty_result():
    result = parse("")
    assert result == XrosReqParseResult(parsed=[], unparsed_lines=[])


def test_named_target_only_digivolve():
    # AD1-001 / BT17-007 shape: "[Digivolve] [Koromon]: Cost 0"
    result = parse("[Digivolve] [Koromon]: Cost 0")
    assert result.unparsed_lines == []
    assert result.parsed == [
        ParsedAltPath(
            kind="digivolve",
            from_={"name_is": "Koromon"},
            materials=None,
            cost=0,
        )
    ]


def test_unrecognized_line_is_unparsed():
    raw = "If 2 such cards are linked together, stack the link card on top and digivolve."
    result = parse(raw)
    assert result.parsed == []
    assert result.unparsed_lines == [raw]


def test_lv_trait_digivolve():
    result = parse("[Digivolve] Lv.5 w/[Xros Heart] trait: Cost 2")
    assert result.unparsed_lines == []
    assert result.parsed == [
        ParsedAltPath(
            kind="digivolve",
            from_={"level_eq": 5, "trait_has": "Xros Heart"},
            materials=None,
            cost=2,
        )
    ]


def test_lv_name_in_name_digivolve():
    result = parse("[Digivolve] Lv.5 w/[Greymon] in name: Cost 3")
    assert result.parsed == [
        ParsedAltPath(
            kind="digivolve",
            from_={"level_eq": 5, "name_contains": "Greymon"},
            materials=None,
            cost=3,
        )
    ]


def test_lv_name_in_text_digivolve():
    # AD1-001: "Lv.3 w/[Omnimon] in text"
    result = parse("[Digivolve] Lv.3 w/[Omnimon] in text: Cost 2")
    assert result.parsed == [
        ParsedAltPath(
            kind="digivolve",
            from_={"level_eq": 3, "name_in_text": "Omnimon"},
            materials=None,
            cost=2,
        )
    ]


def test_multiline_xros_req_each_line_parsed_independently():
    # AD1-004 shape
    raw = (
        "[Digivolve] Lv.5 w/[Greymon] in name: Cost 3\r\n"
        "[Digivolve] Lv.5 w/[Hero] trait: Cost 3"
    )
    result = parse(raw)
    assert result.unparsed_lines == []
    assert len(result.parsed) == 2
    assert result.parsed[0].from_ == {"level_eq": 5, "name_contains": "Greymon"}
    assert result.parsed[1].from_ == {"level_eq": 5, "trait_has": "Hero"}
