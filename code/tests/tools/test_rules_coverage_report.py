import copy
import json
from pathlib import Path

from tools.rules.coverage_report import (
    check_proven_quality,
    check_proven_tests_exist,
    collect_rust_test_fns,
    collect_rust_tests,
    report,
)

REPO = Path(__file__).resolve().parents[3]


def _nothing_proven(obl):
    """The live denominator with every `proven` obligation reset to `gap`.

    Chapter modules mark obligations `proven` as they land, so the tests below
    build on a fixed "nothing proven yet" baseline instead of the live counts.
    """
    out = copy.deepcopy(obl)
    for o in out["obligations"].values():
        if o["status"] == "proven":
            o["status"] = "gap"
            o["tests"] = []
    return out


OBL = _nothing_proven(
    json.loads((REPO / "qa/rules/cr_obligations.json").read_text(encoding="utf-8"))
)


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

RULE = "15-4-3-5-1"


def _tests(tmp_path, src):
    (tmp_path / "m.rs").write_text(src, encoding="utf-8")
    return collect_rust_tests(tmp_path)


def test_collect_rust_tests_reports_ignored_and_cited_ids(tmp_path):
    tests = _tests(
        tmp_path,
        '#[test]\nfn a() { cite!("1-1", "ff"); cite!(\n  "2-2"\n); }\n'
        '#[test]\n#[ignore = "G-X"]\nfn b() { cite!("3-3"); }\n'
        '#[test]\nfn c() {}\n',
    )
    assert tests["m::a"].ignored is False and tests["m::a"].cited_ids == {"1-1", "2-2"}
    assert tests["m::b"].ignored is True and tests["m::b"].cited_ids == {"3-3"}
    assert tests["m::c"].cited_ids == frozenset()


def test_check_passes_when_a_live_test_cites_the_id(tmp_path):
    tests = _tests(tmp_path, f'#[test]\nfn t() {{ cite!("{RULE}", "ff"); }}\n')
    obl = with_status(OBL, RULE, "proven", tests=["m::t"])
    assert check_proven_quality(obl, tests) == ([], [])


def test_check_fails_when_proven_has_no_tests(tmp_path):
    obl = with_status(OBL, RULE, "proven", tests=[])
    errors, _ = check_proven_quality(obl, _tests(tmp_path, ""))
    assert len(errors) == 1 and RULE in errors[0] and "no tests" in errors[0]


def test_check_fails_when_only_listed_test_is_ignored(tmp_path):
    tests = _tests(
        tmp_path, f'#[test]\n#[ignore = "G-X"]\nfn t() {{ cite!("{RULE}", "ff"); }}\n'
    )
    obl = with_status(OBL, RULE, "proven", tests=["m::t"])
    errors, _ = check_proven_quality(obl, tests)
    assert len(errors) == 1 and "non-ignored" in errors[0]


def test_check_fails_when_no_listed_test_cites_the_id(tmp_path):
    tests = _tests(tmp_path, '#[test]\nfn t() { cite!("9-9-9", "ff"); }\n')
    obl = with_status(OBL, RULE, "proven", tests=["m::t"])
    errors, warnings = check_proven_quality(obl, tests)
    assert len(errors) == 1 and RULE in errors[0]
    assert len(warnings) == 1


def test_check_fails_when_no_listed_test_exists(tmp_path):
    obl = with_status(OBL, RULE, "proven", tests=["m::nope"])
    errors, _ = check_proven_quality(obl, _tests(tmp_path, ""))
    assert len(errors) == 1 and "exists" in errors[0]


def test_check_warns_on_secondary_test_that_does_not_cite(tmp_path):
    tests = _tests(
        tmp_path,
        f'#[test]\nfn primary() {{ cite!("{RULE}", "ff"); }}\n'
        '#[test]\nfn secondary() { cite!("9-9-9", "ff"); }\n',
    )
    obl = with_status(OBL, RULE, "proven", tests=["m::primary", "m::secondary"])
    errors, warnings = check_proven_quality(obl, tests)
    assert errors == []
    assert len(warnings) == 1 and "m::secondary" in warnings[0]
