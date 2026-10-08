"""Guards for the vendored Aegis IR snapshot (data/third_party/aegis/).

The snapshot is a LOW-trust structural hint for card authoring; these tests pin
its integrity and make sure the rendered context-pack section keeps its trust
label and never carries requirement data we must not copy.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

import aegis_ir  # noqa: E402


def test_snapshot_matches_pin():
    assert aegis_ir.check() == []


def test_section_carries_low_trust_label():
    section = aegis_ir.aegis_section("BT15-003")
    assert "LOW trust" in section.splitlines()[0]
    assert "never copy its digivolve/Assembly requirement data" in section.splitlines()[0]
    assert '"trigger": "WhenAttacking"' in section


def test_requirement_fields_are_stripped():
    effects = aegis_ir.load_effects()
    for field in aegis_ir.REQUIREMENT_FIELDS:
        cid = next((c for c, rec in effects.items() if field in rec), None)
        if cid is None:
            continue
        assert field not in aegis_ir.aegis_record(cid)
        body = aegis_ir.aegis_section(cid, heading=None).split("```json", 1)[1]
        assert f'"{field}"' not in body


def test_missing_card_is_explicit():
    assert aegis_ir.aegis_record("ZZ9-999") is None
    assert "no Aegis record" in aegis_ir.aegis_section("ZZ9-999")
