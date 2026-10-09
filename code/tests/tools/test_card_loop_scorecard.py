"""Scorecard (tasks 4.5 / 4.6 precision): per-cell metrics on synthetic ledgers, the
lenient-reviewer scenario, Wilson intervals, and the `report models` CLI."""
import json
import math
from datetime import datetime, timezone

import pytest

from tools.card_loop import __main__ as cli
from tools.card_loop import corrections as cx
from tools.card_loop._stats import cost_per_good_unit, intervals_separate, mean_interval, wilson
from tools.card_loop.audit import Audit
from tools.card_loop.contracts import Usage
from tools.card_loop.corrections import CorrectionLog
from tools.card_loop.ledger import Attempt, AttemptLedger
from tools.card_loop.scorecard import Scorecard, cli_report, parse_since, render_table
from tools.card_loop._jsonl import append_row

TS = "2026-10-04T12:00:00.000000Z"


def att(aid, *, stage="implement", family="claude", model="sonnet", pv="implement@1",
        outcome="accepted", cost=1.0, parent=None, ts=TS, usage=None, item=None):
    return Attempt(attempt_id=aid, run_id="r", ts=ts, stage=stage, item=item or f"card:{aid}",
                   family=family, model=model, effort=None, prompt_version=pv,
                   assignment="routed", outcome=outcome,
                   usage=usage if usage is not None else Usage(cost_usd=cost),
                   parent_attempt=parent)


def audit(aid, verdict, sampled=True):
    return Audit(ts=TS, attempt_id=aid, verdict=verdict, sampled=sampled)


# --- statistics helpers -------------------------------------------------------------------

def test_wilson_known_values():
    assert wilson(0, 0) == (0.0, 1.0)
    lo, hi = wilson(5, 10)
    assert lo == pytest.approx(0.2366, abs=1e-4) and hi == pytest.approx(0.7634, abs=1e-4)
    lo, hi = wilson(10, 10)
    assert hi == 1.0 and lo == pytest.approx(0.7225, abs=1e-4)
    with pytest.raises(ValueError):
        wilson(11, 10)


def test_cost_interval_and_separation():
    assert mean_interval([]) == (0.0, 0.0, math.inf)
    assert mean_interval([2.0])[2] == math.inf
    lo, point, hi = cost_per_good_unit([1.0] * 40, 40)
    assert point == 1.0 and lo == pytest.approx(1.0) and 1.0 < hi < 1.1
    assert cost_per_good_unit([1.0, 1.0], 0)[1:] == (math.inf, math.inf)
    assert intervals_separate((0, 1), (2, 3)) == -1
    assert intervals_separate((2, 3), (0, 1)) == 1
    assert intervals_separate((0, 2), (1, 3)) == 0


# --- per-cell metrics ---------------------------------------------------------------------

def _ledger():
    attempts = [
        att("A1", cost=1.0),                               # good
        att("A2", cost=1.0),                               # accepted, later corrected
        att("A3", cost=1.0, outcome="rejected"),           # review reject (immediate)
        att("A4", cost=1.0),                               # accepted, then a gate failed
        att("A5", cost=1.0, outcome="gate_failed"),
        att("C1", family="codex", model=None, cost=0.5),
        att("V1", pv="implement@2", cost=2.0),             # different prompt version
        att("R1", stage="review", family="codex", model=None, pv="review@1", parent="A3",
            cost=0.1),
    ]
    corrections = [
        cx.review_reject("A3", reviewer_attempt="R1", stage="implement", ts=TS),
        cx.gate_fail("A4", gate="cards_behavioral", stage="implement", ts=TS),
        cx.gate_fail("A5", gate="cards_behavioral", stage="implement", ts=TS),
        cx.detect_overturned_verdict({"A2": "x"}, overturned="x", new_verdict="y",
                                     stage="implement", by_human="h", ts=TS)[0],
    ]
    return attempts, corrections


def test_cell_metrics():
    attempts, corrections = _ledger()
    card = Scorecard(attempts, corrections, late_weight=3.0)
    cells = {(c.key.stage, c.key.family, c.key.prompt_version): c for c in card.cells()}
    c = cells[("implement", "claude", "implement@1")]
    assert (c.attempts, c.accepted, c.first_pass) == (5, 3, 2)     # A4 accepted but gate-failed
    assert (c.immediate_corrected, c.late_corrected, c.good_units) == (3, 1, 1)
    assert c.first_pass_rate == pytest.approx(0.4)
    assert c.weighted_correction == pytest.approx((3 + 3 * 1) / 5)
    assert c.cost_total == 5.0 and c.corrected_cost == pytest.approx(5.0)
    assert c.first_pass_ci == wilson(2, 5)
    # prompt versions are never pooled
    assert cells[("implement", "claude", "implement@2")].attempts == 1
    assert cells[("implement", "codex", "implement@1")].corrected_cost == pytest.approx(0.5)
    # stage ordering follows contracts.STAGES
    assert [k[0] for k in cells][-1] == "review"


def test_no_good_units_means_infinite_cost_not_a_crash():
    card = Scorecard([att("A", outcome="rejected"), att("B", outcome="error")])
    (c,) = card.cells()
    assert c.good_units == 0 and math.isinf(c.corrected_cost)
    assert c.to_json()["corrected_cost_usd"] is None
    assert "inf (0 good)" in render_table(card)


def test_unpriced_and_unmetered_attempts():
    tokens_no_cost = Usage(input_tokens=1000, cost_usd=None)
    card = Scorecard([att("A", family="codex", usage=tokens_no_cost), att("B", family="codex")])
    (c,) = card.cells()
    assert c.unpriced_attempts == 1 and c.corrected_cost is None and c.corrected_cost_ci is None
    assert "unpriced" in render_table(card)
    crashed = Scorecard([att("A", outcome="error", usage=Usage()), att("B", cost=2.0)])
    (c,) = crashed.cells()
    assert c.unpriced_attempts == 0 and c.cost_total == 2.0 and c.corrected_cost == 2.0


def test_since_filters_attempts_by_their_timestamp():
    old = att("OLD", ts="2026-08-01T00:00:00Z")
    new = att("NEW", ts="2026-10-03T00:00:00Z")
    now = datetime(2026, 10, 4, tzinfo=timezone.utc)
    card = Scorecard([old, new], since=parse_since("30d", now))
    assert card.cells()[0].attempts == 1
    assert parse_since("2026-09-01", now) == datetime(2026, 9, 1, tzinfo=timezone.utc)
    assert parse_since("2w", now) == datetime(2026, 9, 20, tzinfo=timezone.utc)
    assert parse_since(None) is None
    with pytest.raises(ValueError):
        parse_since("soon", now)


def test_duplicate_rows_from_a_union_merge_count_once():
    a = att("A")
    assert Scorecard([a, a]).cells()[0].attempts == 1


# --- audits: precision and the lenient reviewer -------------------------------------------

def test_audit_precision_uses_sampled_audits_only_but_any_disagree_corrects():
    card = Scorecard([att("A"), att("B"), att("C")],
                     audits=[audit("A", "agree"), audit("B", "disagree"),
                             audit("C", "disagree", sampled=False)])
    (c,) = card.cells()
    assert (c.audited, c.audit_agree) == (2, 1)
    assert c.audit_precision == 0.5
    assert c.good_units == 1           # B and C disagreed


def test_reaudit_last_verdict_wins():
    card = Scorecard([att("A")], audits=[audit("A", "disagree"), audit("A", "agree")])
    assert card.cells()[0].audit_agree == 1


def _lenient_reviewer_ledger():
    attempts, corrections, audits = [], [], []
    # 10 author attempts judged by a lenient Codex reviewer: it approves every one
    for i in range(10):
        a = f"L{i:02d}"
        attempts += [att(a), att(f"RC{i:02d}", stage="review", family="codex", model=None,
                                  pv="review@1", parent=a, cost=0.1)]
    # 10 judged by a strict Claude reviewer: it rejects half
    for i in range(10):
        a = f"S{i:02d}"
        rejected = i % 2 == 1
        attempts += [att(a, outcome="rejected" if rejected else "accepted"),
                     att(f"RS{i:02d}", stage="review", family="claude", model="opus",
                         pv="review@1", parent=a, cost=0.3)]
        if rejected:
            corrections.append(cx.review_reject(a, reviewer_attempt=f"RS{i:02d}",
                                                stage="implement", ts=TS))
    # humans audit five outputs from each reviewer's approvals
    audits += [audit(f"L{i:02d}", "disagree" if i < 4 else "agree") for i in range(5)]
    audits += [audit(f"S{i:02d}", "agree") for i in (0, 2, 4, 6, 8)]
    return attempts, corrections, audits


def test_spec_lenient_reviewer_exposed():
    card = Scorecard(*_lenient_reviewer_ledger())
    by = {(c.key.stage, c.key.family): c for c in card.cells()}
    lenient, strict = by[("review", "codex")], by[("review", "claude")]
    # the scorecard lowers the lenient reviewer's precision ...
    assert (lenient.reviewer_agree, lenient.reviewer_audited) == (1, 5)
    assert lenient.reviewer_precision == pytest.approx(0.2)
    assert strict.reviewer_precision == pytest.approx(1.0)
    # ... and reports the authors' acceptance with and without that reviewer
    infl = {r.reviewer.family: r for r in card.reviewer_influence()}
    r = infl["codex"]
    assert r.author.stage == "implement"
    assert (r.with_rate, r.with_n) == (pytest.approx(15 / 20), 20)
    assert (r.without_rate, r.without_n) == (pytest.approx(5 / 10), 10)
    assert (r.judged, r.approved) == (10, 10)
    assert r.adjusted_rate == pytest.approx((5 + 10 * 0.2) / 20)
    assert infl["claude"].without_rate == pytest.approx(10 / 10)
    text = render_table(card)
    assert "with / without each reviewer" in text and "without 50%" in text
    # the authors' own audit precision reflects the disagreements too
    author = by[("implement", "claude")]
    assert (author.audit_agree, author.audited) == (6, 10)


# --- CLI ----------------------------------------------------------------------------------

@pytest.fixture
def ledgers(tmp_path):
    attempts, corrections, audits = _lenient_reviewer_ledger()
    paths = {k: tmp_path / f"{k}.jsonl" for k in ("attempts", "corrections", "audits")}
    led = AttemptLedger(paths["attempts"])
    for a in attempts:
        led.append(a)
    CorrectionLog(paths["corrections"]).extend(corrections)
    for a in audits:
        append_row(paths["audits"], a.to_row())
    return paths


def _args(paths):
    return ["--attempts", str(paths["attempts"]), "--corrections", str(paths["corrections"]),
            "--audits", str(paths["audits"])]


def test_cli_report_models_json(ledgers, capsys):
    assert cli_report(["models", "--json", *_args(ledgers)]) == 0
    out = json.loads(capsys.readouterr().out)
    assert out["late_correction_weight"] == 3.0 and out["ledger_problems"] == 0
    stages = {(c["stage"], c["family"]) for c in out["cells"]}
    assert stages == {("implement", "claude"), ("review", "codex"), ("review", "claude")}
    impl = next(c for c in out["cells"] if c["stage"] == "implement")
    assert impl["attempts"] == 20 and impl["first_pass"] == 15
    assert len(impl["first_pass_ci"]) == 2
    assert {r["reviewer"] for r in out["reviewer_influence"]} == {"codex/default@review@1",
                                                                   "claude/opus@review@1"}


def test_cli_report_table_stage_filter_and_since(ledgers, capsys):
    assert cli_report(["models", "--stage", "review", *_args(ledgers)]) == 0
    out = capsys.readouterr().out
    assert out.splitlines()[0].startswith("stage") and "implement" not in out
    now = datetime(2027, 1, 1, tzinfo=timezone.utc)
    assert cli_report(["models", "--since", "1d", *_args(ledgers)], now=now) == 0
    assert "no attempts" in capsys.readouterr().out


def test_cli_report_warns_on_damaged_rows(ledgers, capsys):
    with open(ledgers["attempts"], "ab") as f:
        f.write(b"{oops\n")
    assert cli_report(["models", *_args(ledgers)]) == 0
    assert "1 unreadable ledger row" in capsys.readouterr().err


def test_cli_report_usage_errors(ledgers, capsys):
    assert cli_report([]) == 2
    assert cli_report(["models", "--since", "whenever", *_args(ledgers)]) == 2
    assert cli_report(["models", "--stage", "vibes"]) == 2


def test_dispatcher_routes_report_to_the_scorecard(ledgers, capsys):
    assert cli.main(["report", "models", "--json", *_args(ledgers)]) == 0
    assert json.loads(capsys.readouterr().out)["cells"]
