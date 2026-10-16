import copy
import json
from pathlib import Path

from tools.rules.coverage_report import (
    check_proven_tests_exist,
    collect_rust_test_fns,
    report,
)

REPO = Path(__file__).resolve().parents[3]
OBL = json.loads((REPO / "qa/rules/cr_obligations.json").read_text(encoding="utf-8"))


def with_status(obl, rule_id, status, tests=()):
    out = copy.deepcopy(obl)
    out["obligations"][rule_id]["status"] = status
    out["obligations"][rule_id]["tests"] = list(tests)
    return out


def test_report_prints_denominator_and_chapters():
    txt = report(OBL, set())
    assert "| 15 |" in txt and "1119" in txt


def test_report_has_all_chapters_totals_and_mechanics_line():
    txt = report(OBL, set())
    for name in ("Game Overview", "Keyword Effects", "Other Information"):
        assert name in txt
    assert "| Total |" in txt
    assert "Mechanics set:" in txt and "of 715 obligations" in txt


def test_report_counts_proven():
    obl = with_status(OBL, "15-4-3-5-1", "proven", tests=["m::t"])
    assert "1 proven" not in report(OBL, set())
    assert "| 15 | Effect Rules | 244 | 1 |" in report(obl, {"m::t"})


def test_check_fails_when_proven_test_missing():
    obl = with_status(OBL, "15-4-3-5-1", "proven", tests=["ch15_04_effect_states::nope"])
    assert check_proven_tests_exist(
        obl, {"ch15_04_effect_states::turn_player_orders_first"}
    ) == ["ch15_04_effect_states::nope"]


def test_check_passes_when_nothing_proven_or_tests_present():
    assert check_proven_tests_exist(OBL, set()) == []
    obl = with_status(OBL, "15-4-3-5-1", "proven", tests=["m::t"])
    assert check_proven_tests_exist(obl, {"m::t"}) == []


def test_collect_rust_test_fns(tmp_path):
    (tmp_path / "ch99_x.rs").write_text(
        '#[test]\nfn a_b() {}\n#[test]\n#[ignore = "G-X"]\nfn c_d() {}\n'
        '#[test]\n#[should_panic(expected = "x")]\npub fn e_f() {}\nfn not_a_test() {}\n',
        encoding="utf-8",
    )
    assert collect_rust_test_fns(tmp_path) == {"ch99_x::a_b", "ch99_x::c_d", "ch99_x::e_f"}


def test_collect_rust_test_fns_missing_dir(tmp_path):
    assert collect_rust_test_fns(tmp_path / "nope") == set()
