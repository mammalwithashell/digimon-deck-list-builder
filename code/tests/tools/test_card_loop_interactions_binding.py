"""Verdict-store v2 + interaction-scenario binding in `tools.clause_coverage.exam_binding`
(card-loop design D8; tasks 5.2 / 5.3)."""
from __future__ import annotations

import json
from pathlib import Path

import pytest

from tools.clause_coverage.exam_binding import (
    _parse_scenario_header,
    bind,
    bind_interactions,
    load_interaction_verdicts,
    load_verdict_store,
    write_interaction_verdict,
)

ROOT = Path(__file__).resolve().parents[3]


def _clause_row(cid, verdict="confirmed"):
    return {"clause_id": cid, "card_id": cid.split("#")[0], "verdict": verdict,
            "label": "Effect", "text_sha256": "c", "recorded_at": "2026-10-01T00:00:00Z"}


def _iv(iid, cards, verdict="confirmed", sha="r", at="2026-10-02T00:00:00Z", **kw):
    return {"interaction_id": iid, "card_ids": list(cards),
            "source": "qa" if iid.startswith("qa:") else "probe", "kind": "positive",
            "verdict": verdict, "text_sha256": sha, "recorded_at": at, **kw}


def _write(d: Path, card: str, doc: dict) -> None:
    d.mkdir(parents=True, exist_ok=True)
    (d / f"{card}.json").write_text(json.dumps(doc, indent=2), encoding="utf-8")


# --- loaders ------------------------------------------------------------------


def test_v1_and_v2_files_both_load(tmp_path):
    d = tmp_path / "v"
    _write(d, "EX12-073", {"version": 1, "clauses": {"EX12-073#effect#0": _clause_row("EX12-073#effect#0")}})
    _write(d, "BT7-056", {"version": 2, "clauses": {"BT7-056#effect#0": _clause_row("BT7-056#effect#0")},
                          "interactions": {"qa:Q1": _iv("qa:Q1", ["BT7-056"])}})
    assert set(load_verdict_store(d)) == {"EX12-073#effect#0", "BT7-056#effect#0"}
    assert set(load_interaction_verdicts(d)) == {"qa:Q1"}


def test_the_committed_v1_ledger_loads_with_no_interactions():
    d = ROOT / "qa" / "qa-reports" / "exam-verdicts"
    assert load_verdict_store(d)
    assert load_interaction_verdicts(d) == {}


@pytest.mark.parametrize("doc,match", [
    ({"version": 3, "clauses": {}}, "version 3"),
    ({"version": 1, "clauses": {}, "interactions": {"qa:Q1": _iv("qa:Q1", ["BT7-056"])}}, "version 1"),
])
def test_unknown_versions_and_v1_interactions_are_refused(tmp_path, doc, match):
    _write(tmp_path, "BT7-056", doc)
    with pytest.raises(ValueError, match=match):
        load_verdict_store(tmp_path)


def test_a_shared_ruling_merges_once_and_disagreeing_copies_are_refused(tmp_path):
    shared = _iv("qa:Q2", ["BT7-056", "EX4-030"])
    for card in ("BT7-056", "EX4-030"):
        _write(tmp_path, card, {"version": 2, "clauses": {}, "interactions": {"qa:Q2": shared}})
    assert list(load_interaction_verdicts(tmp_path)) == ["qa:Q2"]

    _write(tmp_path, "EX4-030", {"version": 2, "clauses": {},
                                 "interactions": {"qa:Q2": {**shared, "verdict": "diverged"}}})
    with pytest.raises(ValueError, match="disagreeing copies"):
        load_interaction_verdicts(tmp_path)


def test_an_interaction_misfiled_under_a_card_it_does_not_count_for_is_refused(tmp_path):
    _write(tmp_path, "ST1-12", {"version": 2, "clauses": {}, "interactions": {"qa:Q2": _iv("qa:Q2", ["BT7-056"])}})
    with pytest.raises(ValueError, match="ST1-12"):
        load_interaction_verdicts(tmp_path)


def test_an_embedded_id_disagreeing_with_its_key_is_refused(tmp_path):
    _write(tmp_path, "BT7-056", {"version": 2, "clauses": {},
                                 "interactions": {"qa:Q2": _iv("qa:Q9", ["BT7-056"])}})
    with pytest.raises(ValueError, match="qa:Q9"):
        load_interaction_verdicts(tmp_path)


# --- writer -------------------------------------------------------------------


def test_the_writer_files_a_shared_verdict_under_every_card_and_nothing_else(tmp_path):
    _write(tmp_path, "BT7-056", {"version": 1, "last_updated": "2026-10-01T00:00:00Z",
                                 "clauses": {"BT7-056#effect#0": _clause_row("BT7-056#effect#0")}})
    _write(tmp_path, "ST1-12", {"version": 1, "clauses": {"ST1-12#effect#0": _clause_row("ST1-12#effect#0")}})
    untouched = (tmp_path / "ST1-12.json").read_bytes()

    written = write_interaction_verdict(tmp_path, _iv("qa:Q2", ["BT7-056", "EX4-030"], reason=None,
                                                       produced_by="att-7"))
    assert [p.name for p in written] == ["BT7-056.json", "EX4-030.json"]
    assert (tmp_path / "ST1-12.json").read_bytes() == untouched

    bt7 = json.loads((tmp_path / "BT7-056.json").read_text(encoding="utf-8"))
    assert bt7["version"] == 2
    assert list(bt7) == ["version", "last_updated", "clauses", "interactions"]
    assert "BT7-056#effect#0" in bt7["clauses"], "the card's clause verdicts survive"
    row = bt7["interactions"]["qa:Q2"]
    assert list(row)[:6] == ["interaction_id", "card_ids", "source", "kind", "verdict", "text_sha256"]
    assert "reason" not in row and row["produced_by"] == "att-7"
    assert bt7["last_updated"] == "2026-10-02T00:00:00Z"
    assert (tmp_path / "EX4-030.json").read_bytes().count(b"\r") == 0
    assert load_interaction_verdicts(tmp_path)["qa:Q2"]["verdict"] == "confirmed"


def test_the_cross_language_fixture_is_what_the_writer_writes_today(tmp_path):
    """`verdicts_v2/` is read by the Rust test
    `the_python_writers_v2_files_round_trip_byte_identically`; regenerate it
    here so the two writers stay pinned to one format."""
    fixture = Path(__file__).resolve().parent / "fixtures" / "card_loop" / "interactions" / "verdicts_v2"
    clause = {"clause_id": "BT7-056#effect#0", "card_id": "BT7-056", "verdict": "confirmed",
              "label": "Effect", "text_sha256": "c-sha",
              "scenario_path": "qa/dcgo-exams/BT7/BT7-056-effect0.yaml",
              "recorded_at": "2026-10-01T00:00:00Z"}
    _write(tmp_path, "BT7-056", {"version": 1, "last_updated": "2026-10-01T00:00:00Z",
                                 "clauses": {"BT7-056#effect#0": clause}})
    write_interaction_verdict(tmp_path, {
        "interaction_id": "qa:Q2002", "card_ids": ["BT7-056", "EX4-030"], "source": "qa",
        "kind": "positive", "verdict": "confirmed", "text_sha256": "ruling-sha",
        "scenario_path": "qa/dcgo-exams/BT7/BT7-056-qa-Q2002.yaml",
        "recorded_at": "2026-10-02T00:00:00Z", "produced_by": "att-0001"})
    write_interaction_verdict(tmp_path, {
        "interaction_id": "probe:BT7-056#inherited#0:timing_gate:neg", "card_ids": ["BT7-056"],
        "source": "probe", "kind": "negative", "verdict": "diverged", "text_sha256": "clause-sha",
        "reason": "step 4: p0.memory ours=1 dcgo=2", "recorded_at": "2026-10-03T00:00:00Z"})
    for name in ("BT7-056.json", "EX4-030.json"):
        expected = (fixture / name).read_text(encoding="utf-8").replace("\r\n", "\n")
        assert (tmp_path / name).read_text(encoding="utf-8") == expected, name


@pytest.mark.parametrize("bad,match", [
    ({"card_ids": []}, "names no card"),
    ({"verdict": "unmeasured"}, "recordable"),
    ({"verdict": "ours_wrong"}, "recordable"),
    ({"source": None}, "source"),
    ({"recorded_at": ""}, "recorded_at"),
])
def test_the_writer_refuses_incomplete_records(tmp_path, bad, match):
    with pytest.raises(ValueError, match=match):
        write_interaction_verdict(tmp_path, {**_iv("qa:Q2", ["BT7-056"]), **bad})


# --- scenario header ------------------------------------------------------------


def test_the_header_reader_handles_flow_and_block_forms():
    flow = ('card: BT7-056\nclause: BT7-056#effect#0\ncovers: [BT7-056#effect#0, EX4-030#effect#1]\n'
            'interaction: { id: "qa:Q2", source: qa, kind: positive }\nsteps: []\n')
    h = _parse_scenario_header(flow)
    assert h["covers"] == ["BT7-056#effect#0", "EX4-030#effect#1"]
    assert h["interaction"] == {"id": "qa:Q2", "source": "qa", "kind": "positive"}

    block = ("card: BT7-056\nclause: BT7-056#effect#0\ncovers:\n  - BT7-056#effect#0  # primary\n"
             "  - EX4-030#effect#1\ninteraction:\n  id: probe:BT7-056#effect#0:scope:neg\n"
             "  source: probe\n  kind: negative\nseed: 1\n")
    h = _parse_scenario_header(block)
    assert h["covers"] == ["BT7-056#effect#0", "EX4-030#effect#1"]
    assert h["interaction"]["id"] == "probe:BT7-056#effect#0:scope:neg"

    legacy = _parse_scenario_header("card: BT7-056\nclause: BT7-056#effect#0\n")
    assert legacy["covers"] is None and legacy["interaction"] is None


# --- bind(): interaction scenarios ------------------------------------------------


def _scenario(d: Path, name: str, extra: str = "", clause: str = "EX12-073#effect#1") -> Path:
    p = d / f"{name}.yaml"
    p.write_text(f"card: EX12-073\nclause: {clause}\n{extra}seed: 1\ndecks:\n"
                 "  p0: {stack: [], rest: x}\n  p1: {stack: [], rest: x}\n"
                 "steps:\n  - actor: 0\n    do: {pass: {}}\n", encoding="utf-8")
    return p


def _denominator(path: Path, interactions: dict, cards: dict) -> Path:
    path.write_text(json.dumps({"version": 1, "cards": cards, "interactions": interactions}), encoding="utf-8")
    return path


QA_ROW = {"source": "qa", "q_id": "Q7", "card_ids": ["EX12-073"], "kind": "positive",
          "gating": True, "text_sha256": "ruling-sha"}


def test_interaction_scenarios_bind_to_their_interaction_never_to_a_clause(tmp_path):
    sc = tmp_path / "sc"
    sc.mkdir()
    _scenario(sc, "legacy")
    _scenario(sc, "qa", 'interaction: { id: "qa:Q7", source: qa, kind: positive }\n')
    _scenario(sc, "orphan", 'interaction: { id: "qa:Q404", source: qa, kind: positive }\n')
    _scenario(sc, "combo", 'interaction: { id: "combo:x", source: combo, kind: positive }\n')
    den = _denominator(tmp_path / "d.json", {"qa:Q7": QA_ROW}, {"EX12-073": ["qa:Q7"]})

    r = bind(["EX12-073"], sc, tmp_path / "none", interaction_denominator=den)
    by_clause = r["scenarios"]["by_clause"]
    assert [Path(p).stem for p in by_clause["EX12-073#effect#1"]] == ["legacy"]
    assert {k: [Path(p).stem for p in v] for k, v in r["scenarios"]["by_interaction"].items()} == {
        "combo:x": ["combo"], "qa:Q7": ["qa"]}
    kinds = {Path(o["path"]).stem: o["kind"] for o in r["orphan_scenarios"]}
    assert kinds == {"orphan": "orphan_interaction"}
    assert r["scenarios"]["bound"] == 3


def test_without_a_denominator_every_interaction_scenario_is_an_orphan(tmp_path):
    _scenario(tmp_path, "qa", 'interaction: { id: "qa:Q7", source: qa, kind: positive }\n')
    _scenario(tmp_path, "legacy")
    r = bind(["EX12-073"], tmp_path, None, interaction_denominator=tmp_path / "missing.json")
    assert [o["kind"] for o in r["orphan_scenarios"]] == ["denominator_not_generated"]
    assert r["scenarios"]["by_clause"], "legacy scenarios are unaffected"


def test_unknown_covered_clauses_are_orphans_and_known_ones_bind(tmp_path):
    _scenario(tmp_path, "multi", "covers: [EX12-073#effect#1, EX12-073#effect#0]\n")
    _scenario(tmp_path, "typo", "covers: [EX12-073#effect#1, EX12-073#effct#9]\n")
    _scenario(tmp_path, "missing_primary", "covers: [EX12-073#effect#0]\n")
    r = bind(["EX12-073"], tmp_path, None, interaction_denominator=None)
    kinds = sorted((Path(o["path"]).stem, o["kind"]) for o in r["orphan_scenarios"])
    assert kinds == [("missing_primary", "malformed_covers"), ("typo", "unknown_covered_clause")]
    assert any(Path(p).stem == "multi" for p in r["scenarios"]["by_clause"]["EX12-073#effect#0"])


# --- bind_interactions ------------------------------------------------------------


def test_bind_interactions_reports_every_gating_interaction_with_drift(tmp_path):
    den = _denominator(tmp_path / "d.json", {
        "qa:Q7": {**QA_ROW, "card_ids": ["EX12-073", "BT7-056"]},
        "qa:Q8": {**QA_ROW, "q_id": "Q8"},
        "probe:EX12-073#effect#1:scope:neg": {"source": "probe", "card_ids": ["EX12-073"],
                                              "kind": "negative", "gating": False, "text_sha256": "c"},
    }, {"EX12-073": ["qa:Q7", "qa:Q8", "probe:EX12-073#effect#1:scope:neg"],
        "BT7-056": ["qa:Q7"], "ST1-02": []})
    v = tmp_path / "v"
    write_interaction_verdict(v, _iv("qa:Q7", ["EX12-073", "BT7-056"], sha="ruling-sha"))
    write_interaction_verdict(v, _iv("qa:Q8", ["EX12-073"], sha="stale-sha"))

    r = bind_interactions(["EX12-073", "BT7-056", "ST1-02", "XX-999"], den, v)
    assert r["denominator"]["total_interactions"] == 2  # gating only; qa:Q7 counted once
    assert r["denominator"]["by_verdict"]["confirmed"] == 1
    assert r["invalidated_interaction_ids"] == ["qa:Q8"]
    assert r["unmeasured_interaction_ids"] == ["qa:Q8"]
    assert r["cards"]["BT7-056"]["by_verdict"]["confirmed"] == 1
    assert r["cards"]["EX12-073"]["total_interactions"] == 2
    assert r["cards"]["ST1-02"]["total_interactions"] == 0 and r["cards"]["ST1-02"]["in_denominator"]
    assert r["cards_missing_from_denominator"] == ["XX-999"]
    every = bind_interactions(["EX12-073"], den, v, gating_only=False)
    assert every["cards"]["EX12-073"]["total_interactions"] == 3


def test_bind_interactions_refuses_a_missing_denominator(tmp_path):
    with pytest.raises(FileNotFoundError, match="not generated"):
        bind_interactions(["EX12-073"], tmp_path / "missing.json", None)


def test_bind_interactions_surfaces_triage_and_citation_of_a_diverged_row(tmp_path):
    # Readiness (card-loop D9) applies the clause rule to interaction rows: a
    # diverged interaction counts only as a cited dcgo_quirk, so the row must
    # carry the store's `triage` / `citation` twins (verdict-set writes them flat).
    den = _denominator(tmp_path / "d.json", {
        "qa:Q7": QA_ROW,
        "qa:Q8": {**QA_ROW, "q_id": "Q8"},
    }, {"EX12-073": ["qa:Q7", "qa:Q8"]})
    v = tmp_path / "v"
    write_interaction_verdict(v, _iv("qa:Q7", ["EX12-073"], verdict="diverged", sha="ruling-sha",
                                     triage="dcgo_quirk", citation="qa:Q7"))
    write_interaction_verdict(v, _iv("qa:Q8", ["EX12-073"], sha="ruling-sha",
                                     triage="dcgo_quirk", citation="stale"))
    bound = bind_interactions(["EX12-073"], den, v)
    rows = {x["interaction_id"]: x for x in bound["cards"]["EX12-073"]["interactions"]}
    assert (rows["qa:Q7"]["verdict"], rows["qa:Q7"]["triage"], rows["qa:Q7"]["citation"]) == (
        "diverged", "dcgo_quirk", "qa:Q7")
    # not diverged: the twins are None even when the store carries stale text
    assert (rows["qa:Q8"]["verdict"], rows["qa:Q8"]["triage"], rows["qa:Q8"]["citation"]) == (
        "confirmed", None, None)
