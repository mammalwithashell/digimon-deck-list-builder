"""Tests for tools.xros_req_parser."""
from __future__ import annotations

import pytest

from tools.xros_req_parser import (
    ParsedAltPath,
    XrosReqParseResult,
    parse,
    render_alt_paths_yaml,
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


def test_app_fusion_two_materials():
    # AD1-005: "[App Fusion] [Globemon] & [Charismon]: Cost 0"
    result = parse("[App Fusion] [Globemon] & [Charismon]: Cost 0")
    assert result.unparsed_lines == []
    assert result.parsed == [
        ParsedAltPath(
            kind="app_fusion",
            from_=None,
            materials=[{"name_is": "Globemon"}, {"name_is": "Charismon"}],
            cost=0,
        )
    ]


def test_app_fusion_three_materials():
    result = parse("[App Fusion] [A] & [B] & [C]: Cost 1")
    assert result.parsed[0].materials == [
        {"name_is": "A"},
        {"name_is": "B"},
        {"name_is": "C"},
    ]


def test_digixros_requirements_simple():
    # 24 lines like: "DigiXros Requirements [Xros Heart] [Greymon] x 2"
    result = parse("DigiXros Requirements [Xros Heart] [Greymon] x 2")
    assert result.unparsed_lines == []
    assert result.parsed == [
        ParsedAltPath(
            kind="digixros",
            from_=None,
            materials=[{"trait_has": "Xros Heart", "name_is": "Greymon", "count_eq": 2}],
            cost=0,
        )
    ]


def test_descriptor_lines_are_unparsed():
    # 62 lines: "Stack the 2 specified Digimon and digivolve unsuspended."
    raw = "Stack the 2 specified Digimon and digivolve unsuspended."
    result = parse(raw)
    assert result.parsed == []
    assert result.unparsed_lines == [raw]


# ---------------------------------------------------------------------------
# render_alt_paths_yaml tests
# ---------------------------------------------------------------------------

def test_render_empty():
    assert render_alt_paths_yaml([]) == "_(none)_"


def test_render_named_target_only():
    paths = [ParsedAltPath(kind="digivolve", from_={"name_is": "Koromon"}, materials=None, cost=0)]
    assert render_alt_paths_yaml(paths) == (
        "- kind: digivolve\n"
        "  from: { name_is: \"Koromon\" }\n"
        "  cost: 0"
    )


def test_render_lv_trait():
    paths = [ParsedAltPath(
        kind="digivolve", from_={"level_eq": 5, "trait_has": "Xros Heart"},
        materials=None, cost=2,
    )]
    assert render_alt_paths_yaml(paths) == (
        "- kind: digivolve\n"
        "  from: { level_eq: 5, trait_has: \"Xros Heart\" }\n"
        "  cost: 2"
    )


def test_render_amp_materials():
    paths = [ParsedAltPath(
        kind="app_fusion", from_=None,
        materials=[{"name_is": "Globemon"}, {"name_is": "Charismon"}],
        cost=0,
    )]
    assert render_alt_paths_yaml(paths) == (
        "- kind: app_fusion\n"
        "  materials:\n"
        "    - { name_is: \"Globemon\" }\n"
        "    - { name_is: \"Charismon\" }\n"
        "  cost: 0"
    )


def test_render_multiple_paths_separated_by_blank_line():
    paths = [
        ParsedAltPath(kind="digivolve", from_={"level_eq": 5, "name_contains": "Greymon"}, materials=None, cost=3),
        ParsedAltPath(kind="digivolve", from_={"level_eq": 5, "trait_has": "Hero"}, materials=None, cost=3),
    ]
    out = render_alt_paths_yaml(paths)
    assert out == (
        "- kind: digivolve\n"
        "  from: { level_eq: 5, name_contains: \"Greymon\" }\n"
        "  cost: 3\n"
        "- kind: digivolve\n"
        "  from: { level_eq: 5, trait_has: \"Hero\" }\n"
        "  cost: 3"
    )
