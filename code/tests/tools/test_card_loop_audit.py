"""Human audit sample (tasks 4.6): deterministic sampling, `audit next|record`, ledger rows."""
import json

import pytest

from tools.card_loop import __main__ as cli
from tools.card_loop._jsonl import REPO_ROOT
from tools.card_loop.audit import (
    Audit,
    cli_audit,
    default_audits_path,
    in_audit_sample,
    latest_audits,
    load_audits,
    make_audit,
    next_unaudited,
    sampled_attempts,
)
from tools.card_loop.contracts import Usage
from tools.card_loop.ledger import Attempt, AttemptLedger
from tools.card_loop.router import exploration_draw

TS = "2026-10-04T12:00:00.000000Z"


def att(aid, *, stage="implement", outcome="accepted", **kw):
    base = dict(attempt_id=aid, run_id="run-7", ts=TS, stage=stage, item=f"card:{aid}",
                family="claude", model="sonnet", effort=None, prompt_version=f"{stage}@1",
                assignment="routed", outcome=outcome, usage=Usage(cost_usd=0.1))
    base.update(kw)
    return Attempt(**base)


IDS = [f"20261004T{i:06d}000000Z-run-{i:06x}" for i in range(4000)]


def test_sample_is_deterministic_and_about_five_percent():
    picked = [i for i in IDS if in_audit_sample(i, seed=0, rate=0.05)]
    assert picked == [i for i in IDS if in_audit_sample(i, seed=0, rate=0.05)]
    assert 0.035 < len(picked) / len(IDS) < 0.065
    assert not any(in_audit_sample(i, seed=0, rate=0.0) for i in IDS)
    assert all(in_audit_sample(i, seed=0, rate=1.0) for i in IDS)
    # a different seed draws a different sample
    assert picked != [i for i in IDS if in_audit_sample(i, seed=1, rate=0.05)]


def test_audit_draw_is_independent_of_the_router_draw():
    audit_set = {i for i in IDS if in_audit_sample(i, seed=0, rate=0.2)}
    route_set = {i for i in IDS if exploration_draw(0, "implement", i) < 0.2}
    overlap = len(audit_set & route_set) / len(audit_set)
    assert overlap < 0.35          # independent draws overlap ~20%, identical ones 100%


def test_only_accepted_attempts_are_sampled():
    attempts = [att("A"), att("B", outcome="rejected"), att("C", stage="review")]
    got = sampled_attempts(attempts, seed=0, rate=1.0)
    assert [a.attempt_id for a in got] == ["A", "C"]
    assert [a.attempt_id for a in sampled_attempts(attempts, seed=0, rate=1.0,
                                                    stage="review")] == ["C"]


def test_next_unaudited_skips_done_and_goes_oldest_first():
    attempts = [att("B"), att("A"), att("C")]
    audits = [make_audit(attempts[1], "agree", seed=0, rate=1.0, ts=TS)]
    nxt, sampled, done = next_unaudited(attempts, audits, seed=0, rate=1.0)
    assert (nxt.attempt_id, sampled, done) == ("B", 3, 1)


def test_make_audit_row_schema_and_reaudit():
    a = att("A", parent_attempt="P")
    row = make_audit(a, "disagree", seed=0, rate=1.0, auditor="james", note="missed decline",
                     ts=TS).to_row()
    assert row == {"ts": TS, "attempt_id": "A", "verdict": "disagree", "sampled": True,
                   "stage": "implement", "item": "card:A", "family": "claude", "model": "sonnet",
                   "prompt_version": "implement@1", "auditor": "james", "note": "missed decline"}
    with pytest.raises(ValueError):
        make_audit(a, "meh", seed=0, rate=1.0)
    first = Audit(ts=TS, attempt_id="A", verdict="disagree", sampled=True)
    second = Audit(ts=TS, attempt_id="A", verdict="agree", sampled=True)
    assert latest_audits([first, second])["A"].verdict == "agree"


def test_reader_skips_unusable_verdicts(tmp_path):
    p = tmp_path / "audits.jsonl"
    p.write_text("\n".join([
        json.dumps({"ts": TS, "attempt_id": "A", "verdict": "agree", "sampled": True, "x": 1}),
        json.dumps({"ts": TS, "attempt_id": "B", "verdict": "probably"}),
        json.dumps({"verdict": "agree"}),
        "nope",
    ]) + "\n", encoding="utf-8")
    got, problems = load_audits(p)
    assert [a.attempt_id for a in got] == ["A"] and got[0].extra == {"x": 1}
    assert sorted(pr.kind for pr in problems) == ["bad_value", "malformed_json", "missing_keys"]


def test_default_path_is_beside_the_attempt_ledger():
    assert default_audits_path() == REPO_ROOT / "qa" / "card-loop" / "audits.jsonl"


# --- CLI ----------------------------------------------------------------------------------

@pytest.fixture
def env(tmp_path):
    attempts = tmp_path / "attempts.jsonl"
    audits = tmp_path / "audits.jsonl"
    led = AttemptLedger(attempts)
    for a in (att("A1", artifacts={"diff": "runs/card-loop/run-7/A1.patch"}),
              att("A2", stage="author_clause"), att("X1", outcome="rejected")):
        led.append(a)
    cfg = tmp_path / "loop.toml"
    cfg.write_text("audit_rate = 1.0\nseed = 3\n", encoding="utf-8")
    base = ["--config", str(cfg), "--attempts", str(attempts), "--audits", str(audits)]
    return base, audits


def test_cli_next_prints_identity_and_artifact_pointers(env, capsys):
    base, _ = env
    assert cli_audit([*base, "next"]) == 0
    out = capsys.readouterr().out
    assert "A1" in out and "implement" in out and "runs/card-loop/run-7/A1.patch" in out
    assert "run_dir" in out and 'Loop-Attempt: A1' in out
    assert cli_audit([*base, "next", "--stage", "author_clause", "--json"]) == 0
    info = json.loads(capsys.readouterr().out)
    assert info["next"]["attempt_id"] == "A2" and info["sampled"] == 1


def test_cli_record_then_next_advances(env, capsys):
    base, audits = env
    assert cli_audit([*base, "record", "A1", "disagree", "--note", "wrong scope",
                      "--auditor", "james"]) == 0
    got, problems = load_audits(audits)
    assert problems == [] and len(got) == 1
    assert (got[0].attempt_id, got[0].verdict, got[0].sampled, got[0].note, got[0].auditor) == \
        ("A1", "disagree", True, "wrong scope", "james")
    assert b"\r" not in audits.read_bytes()
    capsys.readouterr()
    assert cli_audit([*base, "next", "--json"]) == 0
    assert json.loads(capsys.readouterr().out)["next"]["attempt_id"] == "A2"
    assert cli_audit([*base, "record", "A2", "agree"]) == 0
    capsys.readouterr()
    assert cli_audit([*base, "next"]) == 0
    assert "no unaudited sampled attempts (2/2" in capsys.readouterr().out


def test_cli_record_refusals_and_usage(env, capsys):
    base, audits = env
    assert cli_audit([*base, "record", "NOPE", "agree"]) == 1
    assert cli_audit([*base, "record", "X1", "agree"]) == 1          # rejected: not auditable
    assert cli_audit([*base, "record", "A1", "meh"]) == 2
    assert cli_audit([]) == 2
    assert not audits.exists()


def test_cli_ad_hoc_audit_is_flagged(env, tmp_path, capsys):
    base, audits = env
    cfg = tmp_path / "zero.toml"
    cfg.write_text("audit_rate = 0.0\n", encoding="utf-8")
    args = ["--config", str(cfg), *base[2:]]
    assert cli_audit([*args, "record", "A1", "agree"]) == 0
    assert "ad-hoc" in capsys.readouterr().out
    assert load_audits(audits)[0][0].sampled is False


def test_dispatcher_routes_audit(env, capsys):
    base, _ = env
    assert cli.main(["audit", *base, "next", "--json"]) == 0
    assert json.loads(capsys.readouterr().out)["next"]["attempt_id"] == "A1"
