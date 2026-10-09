"""The D7 three-way outcome table and the DCGO fork-candidate export -- exhaustively."""
from __future__ import annotations

import itertools
import json

import pytest

from tools.card_loop.interactions.outcome import (
    CONFIRMED,
    DCGO_QUIRK,
    OURS_WRONG,
    citation,
    fork_candidates,
    three_way,
    write_fork_candidates,
)

# (ours=DCGO, ours=ruling) -> (verdict_hint, citation_kind, fork, terminating, dcgo=ruling)
TABLE = {
    (True, True): (CONFIRMED, None, False, False, True),
    (True, False): (OURS_WRONG, "qa", True, False, False),
    (False, True): (DCGO_QUIRK, "qa", True, True, False),
    (False, False): (OURS_WRONG, "qa", False, False, None),
}


@pytest.mark.parametrize("legs", list(itertools.product([True, False], repeat=2)))
def test_every_row_of_the_d7_table(legs):
    o = three_way(*legs)
    assert (o.verdict_hint, o.citation_kind, o.dcgo_fork_candidate, o.terminating,
            o.dcgo_matches_ruling) == TABLE[legs]
    assert set(o.to_dict()) >= {"verdict_hint", "citation_kind", "dcgo_fork_candidate"}


def test_the_table_is_exhaustive_and_only_dcgo_quirk_terminates():
    outcomes = {legs: three_way(*legs) for legs in itertools.product([True, False], repeat=2)}
    assert len(outcomes) == 4
    assert [legs for legs, o in outcomes.items() if o.terminating] == [(False, True)]


@pytest.mark.parametrize("bad", [None, 0, 1, "yes"])
def test_an_unmeasured_leg_is_refused_not_coerced(bad):
    with pytest.raises(TypeError):
        three_way(bad, True)
    with pytest.raises(TypeError):
        three_way(True, bad)


def test_citation_is_the_ruling_for_qa_interactions_only():
    assert citation("qa:Q1601", three_way(False, True)) == "qa:Q1601"
    assert citation("qa:Q1601", three_way(True, True)) is None
    assert citation("probe:X#effect#0:scope:neg", three_way(False, True)) is None


def _row(iid, dcgo, ruling, **kw):
    return {"interaction_id": iid, "card_ids": ["EX4-030", "BT7-056"],
            "ours_vs_dcgo_agree": dcgo, "ours_vs_ruling_agree": ruling, **kw}


def test_fork_candidates_are_the_rows_where_dcgo_disagrees_with_the_ruling():
    rows = fork_candidates([
        _row("qa:Q3", True, True),                      # confirmed: no
        _row("qa:Q10", False, True, dcgo_observed="DCGO deleted 2"),  # dcgo_quirk: yes
        _row("qa:Q2", True, False, expected="1 deleted"),  # both wrong vs ruling: yes
        _row("qa:Q4", False, False),                    # ours wrong alone: no
    ])
    assert [r["interaction_id"] for r in rows] == ["qa:Q2", "qa:Q10"]  # natural order
    by = {r["interaction_id"]: r for r in rows}
    assert by["qa:Q10"]["outcome"] == DCGO_QUIRK and by["qa:Q10"]["citation"] == "qa:Q10"
    assert by["qa:Q10"]["dcgo_observed"] == "DCGO deleted 2"
    assert by["qa:Q2"]["outcome"] == OURS_WRONG and by["qa:Q2"]["expected"] == "1 deleted"
    assert by["qa:Q2"]["card_ids"] == ["BT7-056", "EX4-030"]
    assert all(r["source"] == "qa" for r in rows)


def test_a_triaged_probe_quirk_is_a_candidate_and_needs_its_own_citation():
    rows = fork_candidates([
        {"interaction_id": "probe:BT7-056#effect#0:scope:neg", "card_ids": ["BT7-056"],
         "verdict_hint": DCGO_QUIRK, "citation": "general_rule.pdf 15-6"},
        {"interaction_id": "probe:BT7-056#effect#0:leave_play", "card_ids": ["BT7-056"],
         "verdict_hint": OURS_WRONG},
    ])
    assert [r["interaction_id"] for r in rows] == ["probe:BT7-056#effect#0:scope:neg"]
    assert rows[0]["source"] == "probe" and rows[0]["citation"] == "general_rule.pdf 15-6"
    with pytest.raises(ValueError, match="citation"):
        fork_candidates([{"interaction_id": "probe:X#effect#0:scope:neg", "card_ids": ["X"],
                          "verdict_hint": DCGO_QUIRK}])


def test_the_export_is_deterministic_json(tmp_path):
    rows = fork_candidates([_row("qa:Q10", False, True)])
    p = tmp_path / "fork.json"
    write_fork_candidates(rows, p)
    first = p.read_bytes()
    write_fork_candidates(rows, p)
    assert p.read_bytes() == first and b"\r" not in first
    assert json.loads(first)["candidates"][0]["interaction_id"] == "qa:Q10"
