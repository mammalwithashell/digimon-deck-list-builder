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
