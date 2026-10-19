import json
from pathlib import Path

import pytest

from tools.rules.build_obligations import build, check, is_heading, serialise, write

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


def test_invalid_status_raises():
    with pytest.raises(ValueError, match="invalid status"):
        build(SRC, {"1-1": {"status": "done"}}, MECH)


def test_unknown_sidecar_id_raises():
    with pytest.raises(ValueError, match="not in the rules source"):
        build(SRC, {"99-99-99": {"status": "gap"}}, MECH)


def test_non_mapping_sidecar_entry_raises():
    with pytest.raises(ValueError, match="must be mappings"):
        build(SRC, {"1-1": "proven"}, MECH)


def test_stale_mechanism_id_raises():
    with pytest.raises(ValueError, match="mechanisms map"):
        build(SRC, {}, {**MECH, "99-99-99": ["keywords"]})


def test_check_tolerates_crlf_but_not_real_drift(tmp_path):
    crlf = tmp_path / "cr.json"
    crlf.write_bytes(serialise(build(SRC, {}, MECH)).replace(chr(10), chr(13) + chr(10)).encode("utf-8"))
    assert check(crlf, SRC, {}, MECH) == 0
    assert check(crlf, SRC, {"1-1": {"status": "proven", "tests": ["x::y"]}}, MECH) == 1
    assert check(tmp_path / "missing.json", SRC, {}, MECH) == 1


def test_is_heading_branches():
    assert is_heading("Effect States")
    assert not is_heading("This information indicates the card's level.6")  # footnote stripped
    assert is_heading("Infinite Loops44")  # footnote not after a terminator: still a heading
    assert is_heading("<Link +>") and is_heading("<Use Req.>")  # keyword icons
    assert not is_heading("A full sentence that ends with a period.")
    assert not is_heading("Ends with a paren (like this)")
    assert not is_heading("one two three four five six seven")  # > 6 words
    assert not is_heading("Face-down cards placed on the field", "17-1-3-2-4")  # normative prefix
    assert is_heading("Deletion Targets", "17-1-3-1")  # depth-4 sub-heading stays a heading
    assert not is_heading("Deletion Targets", "17-1-3-1-1")  # depth >= 5 leaf
    assert is_heading("Deletion Targets", "17-1-2")


def test_rule_check_leaves_never_seeded_not_testable():
    obs = build(SRC, {}, MECH)["obligations"]
    assert obs["17-1-3-2-4"]["status"] == "gap"
    assert obs["17-1-3-1"]["status"] == "not-testable"  # "Deletion Targets"
    assert obs["17-1-3-2"]["status"] == "not-testable"  # "Trash Targets"
    leaves = [k for k in obs if k.startswith("17-1-3-") and k.count("-") >= 4]
    assert leaves and all(obs[k]["status"] == "gap" for k in leaves)


def test_committed_sidecar_overrides_applied():
    from tools.rules.build_obligations import DEFAULT_SIDECAR, load_sidecar

    sidecar = load_sidecar(DEFAULT_SIDECAR)
    obs = build(SRC, sidecar, MECH)["obligations"]
    assert obs["16-4"]["status"] == "not-testable" and obs["16-4"]["note"]
    # A rule-check leaf seeds as `gap`; whatever the committed sidecar says about
    # it (Plan 4 Task 4 proved 17-1-3-2-4) must be what the build reports.
    assert obs["17-1-3-2-4"]["status"] == sidecar["17-1-3-2-4"]["status"]


def test_summary_mechanics_proven_and_not_testable_counts():
    base = build(SRC, {}, MECH)["summary"]["mechanics"]
    out = build(SRC, {"15-4-3-5-1": {"status": "proven", "tests": ["a::b"]}}, MECH)
    m = out["summary"]["mechanics"]
    assert m["proven"] == 1
    assert m["gap_nonblocking"] == base["gap_nonblocking"] - 1
    assert m["not_testable"] == base["not_testable"] > 0  # headings like 15-4 carry mechanisms
    assert out["summary"]["proven"] == 1
