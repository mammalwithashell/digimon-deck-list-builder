"""The driver's seams (openspec add-card-authoring-loop, design D4/D5/D13)."""
from __future__ import annotations

import pytest

from tools.card_loop import driver_contracts as dc


@pytest.mark.parametrize("kind", ("card", "clause", "interaction"))
def test_every_state_has_a_transition_row_and_terminals_have_no_exits(kind):
    table = dc.TRANSITIONS_BY_KIND[kind]
    assert set(table) == set(dc.STATES_BY_KIND[kind])
    for state in dc.TERMINAL_BY_KIND[kind]:
        assert table[state] == () or state == "UNAVAILABLE"
    for src, dsts in table.items():
        for dst in dsts:
            assert dst in dc.STATES_BY_KIND[kind], (src, dst)


def test_design_d4_paths_are_allowed():
    for src, dst in [("PENDING", "AUTHORING"), ("AUTHORING", "SIM"), ("SIM", "ORACLE"),
                     ("ORACLE", "CONFIRMED"), ("ORACLE", "DIVERGED"), ("DIVERGED", "TRIAGE"),
                     ("TRIAGE", "FIX"), ("FIX", "GATE"), ("GATE", "ORACLE"),
                     ("TRIAGE", "TERMINATION_CHECK"), ("TERMINATION_CHECK", "TERMINAL"),
                     ("TERMINATION_CHECK", "ESCALATED"), ("TRIAGE", "ESCALATED"),
                     ("ORACLE", "AUTHORING"), ("PENDING", "UNAVAILABLE"), ("UNAVAILABLE", "PENDING")]:
        dc.check_transition("clause", src, dst)
    for src, dst in [("PENDING", "IMPLEMENTING"), ("IMPLEMENTING", "REVIEW"),
                     ("REVIEW", "IMPLEMENTED"), ("REVIEW", "IMPLEMENTING"),
                     ("IMPLEMENTING", "PARKED"), ("PARKED", "IMPLEMENTING")]:
        dc.check_transition("card", src, dst)


def test_forbidden_transitions_are_refused():
    with pytest.raises(ValueError, match="not an allowed"):
        dc.check_transition("clause", "CONFIRMED", "AUTHORING")
    with pytest.raises(ValueError, match="not an allowed"):
        dc.check_transition("clause", "ORACLE", "TERMINAL")   # only via a termination check
    with pytest.raises(ValueError, match="not an allowed"):
        dc.check_transition("card", "IMPLEMENTED", "PENDING")
    with pytest.raises(ValueError):
        dc.check_transition("card", "AUTHORING", "SIM")        # not a card state


def test_escalated_is_terminal_but_never_adjudicated():
    for kind in ("clause", "interaction", "card"):
        assert dc.is_terminal(kind, "ESCALATED")
        assert not dc.is_adjudicated(kind, "ESCALATED")
    assert dc.is_adjudicated("clause", "CONFIRMED")
    assert dc.is_adjudicated("clause", "TERMINAL")
    assert dc.is_adjudicated("clause", "UNAVAILABLE")
    assert dc.is_adjudicated("card", "IMPLEMENTED")
    assert not dc.is_adjudicated("card", "PARKED")


def test_stage_for_state_follows_the_item_kind_and_the_fix_target():
    assert dc.stage_for("clause", "AUTHORING") == "author_clause"
    assert dc.stage_for("interaction", "AUTHORING") == "author_interaction"
    assert dc.stage_for("clause", "FIX") == "fix_card"
    assert dc.stage_for("clause", "FIX", engine_fix=True) == "fix_engine"
    assert dc.stage_for("interaction", "CLASSIFY") == "classify_qa"
    assert dc.stage_for("card", "REVIEW") == "review"
    assert dc.stage_for("clause", "ORACLE") is None
    assert dc.stage_for("clause", "GATE") is None


def test_records_round_trip():
    rec = dc.ItemRecord(item="clause:BT7-056#effect#0", state="SIM",
                        attempts={"author_clause": 1}, data={"scenario": "x.yaml"})
    assert rec.kind == "clause" and rec.ident == "BT7-056#effect#0"
    assert dc.ItemRecord.from_dict(rec.to_dict()) == rec
    ev = dc.Event(ts="2026-10-05T00:00:00Z", run_id="r", item=rec.item, src="AUTHORING",
                  dst="SIM", reason="lint clean", stage="author_clause", attempt_id="a1",
                  data={"paths": ["x.yaml"]})
    assert dc.Event.from_row(ev.to_row()) == ev
    bare = dc.Event(ts="t", run_id="r", item=rec.item, src=None, dst="PENDING")
    assert "stage" not in bare.to_row() and "data" not in bare.to_row()
