"""Tests for the relocated DNA/DigiXros cost parser (`tools.xros_cost_parser`).

The parser was ported off `engine_py_legacy` (shrink-legacy-engine-surface,
task 5.3) so `tools/ingest_cards.py` is legacy-free. Two layers:

1. Durable smoke assertions on the new parser alone.
2. A byte-identical parity gate against the legacy parser over every
   `xros_req` string in `cards.json` — guarded by `importorskip` so it
   disappears cleanly when `engine_py_legacy` is deleted.
"""

from __future__ import annotations

import json

import pytest

from tools.xros_cost_parser import parse_digixros_req, parse_xros_req


def test_dna_parse_smoke():
    costs = parse_xros_req("[DNA Digivolve] Blue Lv.4 + Green Lv.4: Cost 0")
    assert len(costs) == 1
    c = costs[0]
    assert c.memory_cost == 0
    assert c.requirement1.level == 4
    assert [col.name for col in c.requirement1.card_colors] == ["Blue"]
    assert [col.name for col in c.requirement2.card_colors] == ["Green"]


def test_dna_parse_slash_color_and_name_constraint():
    costs = parse_xros_req(
        "[DNA Digivolve] Blue/Purple Lv.6 + Lv.6 w/[Greymon] in name: Cost 0"
    )
    assert len(costs) == 1
    assert [col.name for col in costs[0].requirement1.card_colors] == ["Blue", "Purple"]
    assert costs[0].requirement2.name_contains == "Greymon"


def test_digixros_parse_smoke():
    dx = parse_digixros_req("[DigiXros -3] [Shoutmon] x [Ballistamon] x [Dorulumon]")
    assert len(dx) == 1
    assert dx[0].reduce_cost_per_card == 3
    assert [e.name_contains for e in dx[0].elements] == [
        "Shoutmon",
        "Ballistamon",
        "Dorulumon",
    ]


def test_no_xros_block_returns_empty():
    assert parse_xros_req("Some unrelated effect text") == []
    assert parse_digixros_req("") == []


def test_byte_identical_with_legacy_over_cards_json():
    """The relocated parser must produce output identical to the legacy parser
    (via the same ingest JSON serializers) for every real `xros_req` string."""
    pytest.importorskip("engine_py_legacy.engine.data.card_database")
    from data_paths import CARDS_JSON
    from engine_py_legacy.engine.data.card_database import (
        parse_digixros_req as legacy_digixros,
        parse_xros_req as legacy_dna,
    )
    from tools.ingest_cards import _digixros_cost_to_json, _dna_cost_to_json

    cards = json.load(open(CARDS_JSON, encoding="utf-8"))
    xros_strings = sorted({c["xros_req"] for c in cards.values() if c.get("xros_req")})
    assert xros_strings, "expected some xros_req strings in cards.json"

    mismatches = []
    for s in xros_strings:
        new_dna = [_dna_cost_to_json(dc) for dc in parse_xros_req(s)]
        leg_dna = [_dna_cost_to_json(dc) for dc in legacy_dna(s)]
        new_dx = [_digixros_cost_to_json(x) for x in parse_digixros_req(s)]
        leg_dx = [_digixros_cost_to_json(x) for x in legacy_digixros(s)]
        if new_dna != leg_dna or new_dx != leg_dx:
            mismatches.append(s)

    assert not mismatches, f"{len(mismatches)} xros_req strings diverged from legacy: {mismatches[:5]}"
