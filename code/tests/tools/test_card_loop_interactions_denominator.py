"""The gating interaction denominator (design D6; spec "interaction-exams").

Built from a fixture Q&A mirror plus the REAL committed card data for a
four-card universe (fixtures/card_loop/interactions/cards.txt).
"""
from __future__ import annotations

import json
import shutil
from pathlib import Path

import pytest

from tools.card_loop import __main__ as card_loop_cli
from tools.card_loop.interactions import denominator as den
from tools.card_loop.interactions.probes import (
    FAMILY_VERSIONS,
    POSITIVE,
    Family,
)

FIX = Path(__file__).resolve().parent / "fixtures" / "card_loop" / "interactions"
CARD_QA = FIX / "card_qa.json"
CARDS = FIX / "cards.txt"
UNIVERSE = ["BT7-056", "EX4-030", "BT21-029", "ST1-02"]


@pytest.fixture(scope="module")
def clauses() -> list[dict]:
    return den.extract_clauses(UNIVERSE)


@pytest.fixture(scope="module")
def card_qa() -> dict:
    return den.load_card_qa(CARD_QA)


def build(clauses, card_qa, promoted=None, registry=FAMILY_VERSIONS, universe=UNIVERSE):
    return den.build_denominator(
        universe, clauses, card_qa,
        promoted if promoted is not None else {"families@1": True},
        registry=registry,
    )


# --- shape and determinism -----------------------------------------------------


def test_the_same_inputs_give_byte_identical_output(clauses, card_qa):
    a = den.render(build(clauses, card_qa))
    b = den.render(build(list(reversed(clauses)), card_qa, universe=list(reversed(UNIVERSE))))
    assert a == b
    assert "\r" not in a and a.endswith("}\n")
    assert a.isascii()


def test_the_artifact_has_the_documented_shape(clauses, card_qa):
    art = build(clauses, card_qa)
    assert art["version"] == 1
    assert art["families"]["families@1"]["promoted"] is True
    assert set(art["families"]["families@1"]["families"]) >= {"optional_decline", "timing_gate"}
    assert set(art["cards"]) == set(UNIVERSE)
    for iid, rec in art["interactions"].items():
        assert rec["source"] in ("qa", "probe")
        assert rec["kind"] in ("positive", "negative")
        assert len(rec["text_sha256"]) == 64
        assert rec["card_ids"]
        if rec["source"] == "qa":
            assert iid == "qa:" + rec["q_id"]
        else:
            assert iid.startswith(f"probe:{rec['clause_id']}:{rec['family']}")
            assert rec["family_version"] == "families@1"


def test_a_shared_ruling_is_one_interaction_counted_for_both_cards(clauses, card_qa):
    art = build(clauses, card_qa)
    assert art["interactions"]["qa:Q2002"]["card_ids"] == ["BT7-056", "EX4-030"]
    assert "qa:Q2002" in art["cards"]["BT7-056"]
    assert "qa:Q2002" in art["cards"]["EX4-030"]
    assert sum(1 for i in art["interactions"] if i == "qa:Q2002") == 1


def test_each_cards_list_is_rulings_in_q_order_then_probes(clauses, card_qa):
    ids = build(clauses, card_qa)["cards"]["BT7-056"]
    qa = [i for i in ids if i.startswith("qa:")]
    assert qa == ["qa:Q999", "qa:Q2001", "qa:Q2002"]  # natural, not lexical, order
    assert ids[:3] == qa
    assert all(i.startswith("probe:BT7-056#") for i in ids[3:]) and ids[3:]


def test_a_vanilla_card_is_listed_with_no_interactions(clauses, card_qa):
    # Ready on its clauses alone -- distinguishable from "never considered".
    art = build(clauses, card_qa)
    assert art["cards"]["ST1-02"] == []


def test_model_authored_combos_never_enter_the_denominator(clauses, card_qa):
    art = build(clauses, card_qa)
    assert not any(i.startswith("combo:") for i in art["interactions"])


def test_rulings_and_promoted_probes_gate(clauses, card_qa):
    art = build(clauses, card_qa)
    assert all(r["gating"] for r in art["interactions"].values())
    assert art["summary"]["gating_interactions"] == art["summary"]["interactions"]


def test_an_unpromoted_family_is_generated_but_does_not_gate(clauses, card_qa):
    art = build(clauses, card_qa, promoted={"families@1": False})
    probes = [r for r in art["interactions"].values() if r["source"] == "probe"]
    assert probes and not any(r["gating"] for r in probes)
    assert all(r["gating"] for r in art["interactions"].values() if r["source"] == "qa")


def test_a_card_qa_reference_to_a_missing_ruling_is_refused(clauses):
    bad = {"cards": {"BT7-056": ["Q1"]}, "qa": {}}
    with pytest.raises(ValueError, match="Q1"):
        den.build_denominator(UNIVERSE, clauses, bad, {"families@1": True})


# --- promotion -----------------------------------------------------------------


def _with_toy_v2():
    reg = dict(FAMILY_VERSIONS)
    reg["families@2"] = (
        Family("names_digimon", POSITIVE, lambda v: "text:digimon" if "digimon" in v.body else None),
    )
    return reg


def test_a_new_family_version_gates_only_after_promotion(clauses, card_qa):
    reg = _with_toy_v2()
    art = build(clauses, card_qa, promoted={"families@1": True}, registry=reg)
    v2 = {i: r for i, r in art["interactions"].items() if r.get("family_version") == "families@2"}
    assert v2, "the toy family should fire on the fixture cards"
    assert not any(r["gating"] for r in v2.values())

    report = den.promotion_report(art, "families@2")
    assert report["promoted"] is False
    assert report["new_gating_probes"] == len(v2)
    assert set(report["cards"]) == {c for r in v2.values() for c in r["card_ids"]}
    assert report["cards_that_would_drop"] == list(report["cards"])  # natural order, all drop

    promoted = build(clauses, card_qa, promoted={"families@1": True, "families@2": True}, registry=reg)
    assert all(promoted["interactions"][i]["gating"] for i in v2)
    assert den.promotion_report(promoted, "families@2")["new_gating_probes"] == 0


def test_promotion_report_discounts_already_adjudicated_probes(clauses, card_qa):
    reg = _with_toy_v2()
    art = build(clauses, card_qa, registry=reg)
    report = den.promotion_report(art, "families@2")
    card, ids = next(iter(report["cards"].items()))
    verdicts = {i: {"verdict": "confirmed"} for i in ids}
    again = den.promotion_report(art, "families@2", verdicts)
    assert card not in again["cards_that_would_drop"]


def test_load_promotion_defaults_unlisted_versions_to_unpromoted(tmp_path):
    reg = _with_toy_v2()
    p = tmp_path / "promotion.json"
    p.write_text(json.dumps({"families@1": {"promoted": True}}), encoding="utf-8")
    assert den.load_promotion(p, reg) == {"families@1": True, "families@2": False}
    p.write_text(json.dumps({"families@9": {"promoted": True}}), encoding="utf-8")
    with pytest.raises(ValueError, match="families@9"):
        den.load_promotion(p, reg)


def test_the_committed_promotion_file_promotes_the_base_families():
    assert den.load_promotion(den.DEFAULT_PROMOTION) == {"families@1": True}


# --- CLI: build / --check -------------------------------------------------------


def _args(out: Path, qa: Path = CARD_QA, *extra: str) -> list[str]:
    return ["--out", str(out), "--card-qa", str(qa), "--cards-from", str(CARDS), *extra]


def test_build_then_check_is_clean(tmp_path, capsys):
    out = tmp_path / "interaction_denominator.json"
    assert den.cli(["build", *_args(out)]) == 0
    assert out.read_bytes().count(b"\r") == 0
    assert den.cli(["--check", *_args(out)]) == 0
    assert "OK" in capsys.readouterr().out


def test_check_fails_naming_a_ruling_added_without_regenerating(tmp_path, capsys):
    out = tmp_path / "interaction_denominator.json"
    assert den.cli(["build", *_args(out)]) == 0
    qa = json.loads(CARD_QA.read_text(encoding="utf-8"))
    qa["cards"]["ST1-02"] = ["Q3000"]
    qa["qa"]["Q3000"] = {"q_id": "Q3000", "date": None, "question": "q?", "answer": "a.",
                         "card_ids": ["ST1-02"]}
    drifted = tmp_path / "card_qa.json"
    drifted.write_text(json.dumps(qa), encoding="utf-8")
    capsys.readouterr()

    assert den.cli(["check", *_args(out, drifted)]) == 1
    text = capsys.readouterr().out
    assert "DRIFT" in text and "qa:Q3000" in text


def test_check_names_removed_and_changed_rulings(tmp_path, capsys):
    out = tmp_path / "interaction_denominator.json"
    assert den.cli(["build", *_args(out)]) == 0
    qa = json.loads(CARD_QA.read_text(encoding="utf-8"))
    del qa["qa"]["Q2003"]
    qa["cards"]["BT21-029"] = []
    qa["qa"]["Q2001"]["answer"] = "Yes, you may choose to add none."
    drifted = tmp_path / "card_qa.json"
    drifted.write_text(json.dumps(qa), encoding="utf-8")
    capsys.readouterr()

    assert den.cli(["--check", *_args(out, drifted)]) == 1
    text = capsys.readouterr().out
    assert "removed interactions (1)" in text and "qa:Q2003" in text
    assert "changed interactions (1)" in text and "qa:Q2001" in text


def test_check_without_a_committed_artifact_fails_as_not_generated(tmp_path, capsys):
    assert den.cli(["--check", *_args(tmp_path / "missing.json")]) == 1
    assert "not generated" in capsys.readouterr().err


def test_check_reuses_a_committed_card_scoped_universe(tmp_path):
    out = tmp_path / "interaction_denominator.json"
    assert den.cli(["build", *_args(out)]) == 0
    art = json.loads(out.read_text(encoding="utf-8"))
    assert art["universe"]["mode"] == "cards"
    # No --cards-from: the check rebuilds over the committed card list.
    assert den.cli(["--check", "--out", str(out), "--card-qa", str(CARD_QA)]) == 0


def test_the_card_loop_dispatcher_routes_interactions(tmp_path):
    out = tmp_path / "d.json"
    assert card_loop_cli.main(["interactions", "build", *_args(out)]) == 0
    assert out.exists()


def test_promotion_report_cli_says_when_everything_is_promoted(tmp_path, capsys):
    out = tmp_path / "d.json"
    assert den.cli(["build", *_args(out)]) == 0
    capsys.readouterr()
    assert den.cli(["promotion-report", "--out", str(out), "--verdicts", str(tmp_path / "none")]) == 0
    assert "already promoted" in capsys.readouterr().out


def test_promotion_report_cli_lists_cards_an_unpromoted_version_would_drop(tmp_path, capsys):
    out = tmp_path / "d.json"
    unpromoted = FIX / "promotion_unpromoted.json"
    assert den.cli(["build", *_args(out), "--promotion", str(unpromoted)]) == 0
    capsys.readouterr()
    report = tmp_path / "report.json"
    assert den.cli(["promotion-report", "--out", str(out), "--verdicts", str(tmp_path / "none"),
                    "--json", str(report)]) == 0
    text = capsys.readouterr().out
    assert "families@1 (promoted: False)" in text
    rows = json.loads(report.read_text(encoding="utf-8"))
    assert {"BT7-056", "EX4-030", "BT21-029"} <= set(rows[0]["cards_that_would_drop"])
    assert "ST1-02" not in rows[0]["cards"]


def test_read_card_ids_accepts_json_lists_and_text(tmp_path):
    a = tmp_path / "a.json"
    a.write_text(json.dumps(["ST1-12", "BT1-001", "ST1-12"]), encoding="utf-8")
    b = tmp_path / "b.json"
    b.write_text(json.dumps({"cards": {"EX4-030": [], "BT7-056": []}}), encoding="utf-8")
    assert den.read_card_ids(a) == ["BT1-001", "ST1-12"]
    assert den.read_card_ids(b) == ["BT7-056", "EX4-030"]
    assert den.read_card_ids(CARDS) == ["BT7-056", "BT21-029", "EX4-030", "ST1-02"]


def test_the_committed_fixture_denominator_matches_the_builders_format(clauses, card_qa):
    """`fixtures/.../interaction_denominator.json` is what the Rust lint test
    (`the_python_built_denominator_drives_the_orphan_rule`) reads; keep it in
    the builder's current format. Regenerate with
    `python -m tools.card_loop interactions build --out <fixture> --card-qa
    <fixture card_qa> --cards-from <fixture cards.txt>`."""
    committed = den.load_denominator(FIX / "interaction_denominator.json")
    fresh = build(clauses, card_qa)
    assert set(committed) == set(fresh)
    assert committed["universe"] == {"mode": "cards",
                                     "source": "code/tests/tools/fixtures/card_loop/interactions/cards.txt"}
    qa = lambda a: {i: r for i, r in a["interactions"].items() if i.startswith("qa:")}  # noqa: E731
    assert qa(committed) == qa(fresh)
    assert set(committed["cards"]) == set(UNIVERSE)


def test_the_module_entry_point_exists():
    import runpy
    import sys

    argv = sys.argv
    try:
        sys.argv = ["x", "--help"]
        with pytest.raises(SystemExit) as e:
            runpy.run_module("tools.card_loop.interactions", run_name="__main__")
        assert e.value.code == 0
    finally:
        sys.argv = argv
