import json
from pathlib import Path

from tools.rules.build_obligations import build, check, write

REPO = Path(__file__).resolve().parents[3]
SRC = REPO / "qa/rules/aegis/comprehensive-4.3.md"
MECH = json.loads(
    (REPO / "qa/rules/aegis/mechanisms-by-rule.json").read_text(encoding="utf-8")
)


def test_default_status_is_gap_and_mechanisms_attached(tmp_path):
    out = build(source=SRC, sidecar={}, mechanisms=MECH)
    o = out["obligations"]["15-4-3-5-1"]
    assert o["status"] == "gap" and o["blocks_training"] is False
    assert "effects-and-ordering" in o["mechanisms"]
    assert o["tests"] == [] and o["gap_id"] is None and o["note"] is None


def test_sidecar_overrides_status_and_tests():
    out = build(
        SRC,
        {
            "15-4-3-5-1": {
                "status": "proven",
                "tests": ["ch15_04_effect_states::turn_player_orders_first"],
            }
        },
        MECH,
    )
    o = out["obligations"]["15-4-3-5-1"]
    assert o["status"] == "proven"
    assert o["tests"] == ["ch15_04_effect_states::turn_player_orders_first"]


def test_not_testable_seeded_for_headings():
    out = build(SRC, {}, MECH)
    assert out["obligations"]["15-4"]["status"] == "not-testable"  # "Effect States"
    assert out["obligations"]["15-4-3-5-1"]["status"] == "gap"  # a real sentence


def test_summary_mechanics_counts():
    out = build(SRC, {"13-1-2": {"status": "gap", "blocks_training": True}}, MECH)
    assert out["summary"]["mechanics"]["gap_blocking"] == 1


def test_check_detects_drift(tmp_path):
    write(tmp_path / "cr.json", build(SRC, {}, MECH))
    assert check(tmp_path / "cr.json", SRC, {}, MECH) == 0
    assert (
        check(tmp_path / "cr.json", SRC, {"1-1": {"status": "proven", "tests": ["x::y"]}}, MECH)
        == 1
    )
