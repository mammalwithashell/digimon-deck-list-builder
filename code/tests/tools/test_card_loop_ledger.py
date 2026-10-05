"""Attempt ledger (tasks 4.1): schema, LF append writer, tolerant reader, union merge."""
import json
import os
import subprocess
import threading
from datetime import datetime, timezone

import pytest

from tools.card_loop import ledger
from tools.card_loop._jsonl import REPO_ROOT, append_row, read_rows
from tools.card_loop.contracts import TaskPacket, Usage, WorkerResult
from tools.card_loop.ledger import (
    ASSIGNMENTS,
    OUTCOMES,
    Attempt,
    AttemptLedger,
    attempt_from_call,
    dedupe_last,
    load_attempts,
    new_attempt_id,
)


def _attempt(**over):
    base = dict(attempt_id=new_attempt_id("run1"), run_id="run1", ts="2026-10-04T12:00:00.000000Z",
                stage="implement", item="card:BT7-056", family="claude", model="sonnet",
                effort=None, prompt_version="implement@1", assignment="routed",
                outcome="accepted", usage=Usage(input_tokens=10, output_tokens=5, cost_usd=0.25))
    base.update(over)
    return Attempt(**base)


def test_vocabularies_match_the_documented_contract():
    assert OUTCOMES == ("accepted", "rejected", "gate_failed", "schema_invalid", "error",
                        "quota_exhausted", "escalated")
    assert ASSIGNMENTS == ("routed", "explore", "forced")
    for word in OUTCOMES + ASSIGNMENTS:
        assert word in ledger.__doc__


def test_round_trip_and_row_shape(tmp_path):
    led = AttemptLedger(tmp_path / "attempts.jsonl")
    a = _attempt(parent_attempt="p1", artifacts={"diff": "runs/x/d.patch"}, notes="n")
    led.append(a)
    got, problems = led.read()
    assert problems == []
    assert got == [a]
    row = json.loads((tmp_path / "attempts.jsonl").read_text(encoding="utf-8"))
    assert list(row)[:12] == ["attempt_id", "run_id", "ts", "stage", "item", "family", "model",
                              "effort", "prompt_version", "assignment", "outcome", "usage"]
    assert row["usage"] == {"input_tokens": 10, "output_tokens": 5, "cache_read_tokens": 0,
                            "cache_write_tokens": 0, "cost_usd": 0.25, "cost_derived": False,
                            "wall_seconds": 0.0}


def test_optional_keys_are_omitted_when_empty(tmp_path):
    p = tmp_path / "a.jsonl"
    AttemptLedger(p).append(_attempt())
    row = json.loads(p.read_text(encoding="utf-8"))
    assert "parent_attempt" not in row and "artifacts" not in row and "notes" not in row


def test_writer_emits_compact_lf_lines(tmp_path):
    p = tmp_path / "a.jsonl"
    led = AttemptLedger(p)
    led.append(_attempt())
    led.append(_attempt())
    raw = p.read_bytes()
    assert b"\r" not in raw
    assert raw.count(b"\n") == 2 and raw.endswith(b"\n")
    assert b", " not in raw and b'": ' not in raw  # compact separators


def test_writer_rejects_bad_vocabulary(tmp_path):
    led = AttemptLedger(tmp_path / "a.jsonl")
    for bad in ({"outcome": "maybe"}, {"assignment": "random"}, {"stage": "vibes"},
                {"family": "gemini"}, {"attempt_id": "has space"}, {"prompt_version": ""}):
        with pytest.raises(ValueError):
            led.append(_attempt(**bad))
    assert not (tmp_path / "a.jsonl").exists()


def test_concurrent_appends_in_one_process_stay_line_atomic(tmp_path):
    p = tmp_path / "a.jsonl"
    led = AttemptLedger(p)
    n_threads, per = 8, 50

    def work(t):
        for i in range(per):
            led.append(_attempt(notes=f"t{t}-{i}" + "x" * 200))

    threads = [threading.Thread(target=work, args=(t,)) for t in range(n_threads)]
    for th in threads:
        th.start()
    for th in threads:
        th.join()
    got, problems = load_attempts(p)
    assert problems == []
    assert len(got) == n_threads * per
    assert len({a.attempt_id for a in got}) == n_threads * per


def test_reader_tolerates_drift_and_flags_damage(tmp_path):
    p = tmp_path / "a.jsonl"
    good = _attempt().to_row()
    future = {**_attempt().to_row(), "outcome": "superseded", "new_field": {"x": 1},
              "usage": {"input_tokens": 3, "gpu_seconds": 9}}
    no_id = {k: v for k, v in _attempt().to_row().items() if k != "attempt_id"}
    lines = [json.dumps(good), "", "{not json", "[1, 2]", json.dumps(future), json.dumps(no_id)]
    # CRLF + BOM, as a Windows editor might leave it
    p.write_bytes(("﻿" + "\r\n".join(lines) + "\r\n").encode("utf-8"))
    got, problems = load_attempts(p)
    assert [a.attempt_id for a in got] == [good["attempt_id"], future["attempt_id"]]
    assert got[1].extra == {"new_field": {"x": 1}}
    assert got[1].usage.input_tokens == 3
    assert got[1].to_row()["new_field"] == {"x": 1}          # unknown keys survive a round trip
    kinds = sorted((pr.line, pr.kind) for pr in problems)
    assert kinds == [(3, "malformed_json"), (4, "not_object"), (5, "bad_value"),
                     (6, "missing_keys")]


def test_missing_file_is_an_empty_ledger(tmp_path):
    assert load_attempts(tmp_path / "nope.jsonl") == ([], [])


def test_append_after_a_torn_line_isolates_the_damage(tmp_path):
    p = tmp_path / "a.jsonl"
    p.write_bytes(b'{"attempt_id": "half')     # a writer died mid-line
    AttemptLedger(p).append(_attempt())
    got, problems = load_attempts(p)
    assert len(got) == 1 and [pr.kind for pr in problems] == ["malformed_json"]


def test_attempt_ids_are_unique_sortable_and_trailer_safe(monkeypatch):
    monkeypatch.setattr(ledger, "_last_stamp", [])   # don't leak a 2041 clock to other tests
    # Future instants: the in-process clock is monotonic, so a `now` earlier than
    # an id already issued by another test would (correctly) be bumped forward.
    t = datetime(2040, 10, 4, 12, 0, 0, tzinfo=timezone.utc)
    ids = [new_attempt_id("pilot run/7", now=t) for _ in range(200)]   # same instant
    assert len(set(ids)) == 200
    assert ids == sorted(ids)                       # strictly increasing within a process
    assert all(not any(c.isspace() for c in i) and "/" not in i for i in ids)
    assert ids[0].startswith("20401004T120000000000Z-pilot_run_7-")
    later = new_attempt_id("a", now=datetime(2041, 1, 1, tzinfo=timezone.utc))
    assert later > ids[-1]
    earlier = new_attempt_id("a", now=datetime(2001, 1, 1, tzinfo=timezone.utc))
    assert earlier > later                          # never goes backwards in-process


def test_attempt_from_call_maps_packet_and_result():
    packet = TaskPacket(stage="review", family="codex", item="card:BT7-056", attempt_id="A2",
                        prompt="p", prompt_version="review@3", schema_path="s.json",
                        worktree="wt", model="gpt-x", effort="high")
    ok = WorkerResult(status="ok", artifacts={"diff": "d.patch"}, transcript_path="t.jsonl",
                      usage=Usage(cost_usd=None, cost_derived=False, input_tokens=7))
    with pytest.raises(ValueError):
        attempt_from_call(packet, ok, run_id="r", assignment="routed")   # driver must decide
    a = attempt_from_call(packet, ok, run_id="r", assignment="explore", outcome="accepted",
                          parent_attempt="A1")
    assert (a.stage, a.family, a.model, a.effort, a.prompt_version) == \
        ("review", "codex", "gpt-x", "high", "review@3")
    assert a.artifacts == {"diff": "d.patch", "transcript": "t.jsonl"}
    assert a.parent_attempt == "A1" and a.usage.input_tokens == 7
    a.validate()
    quota = WorkerResult(status="quota_exhausted", error="credit balance too low")
    q = attempt_from_call(packet, quota, run_id="r", assignment="routed")
    assert q.outcome == "quota_exhausted" and q.notes == "credit balance too low"


def test_dedupe_keeps_last_occurrence_in_first_seen_order():
    a1, b = _attempt(attempt_id="A"), _attempt(attempt_id="B")
    a2 = _attempt(attempt_id="A", outcome="rejected")
    assert dedupe_last([a1, b, a2]) == [a2, b]


def test_default_path_comes_from_config_and_anchors_at_repo_root():
    led = AttemptLedger()
    assert led.path == REPO_ROOT / "qa" / "card-loop" / "attempts.jsonl"


def test_repo_gitattributes_union_merges_every_ledger():
    text = (REPO_ROOT / ".gitattributes").read_text(encoding="utf-8")
    for name in ("attempts", "corrections", "audits"):
        assert f"qa/card-loop/{name}.jsonl merge=union" in text


# --- spec scenario: concurrent runs on different branches merge cleanly -------------------

def _git(cwd, *args, env):
    return subprocess.run(["git", *args], cwd=cwd, env=env, check=True,
                          capture_output=True, text=True, encoding="utf-8").stdout


@pytest.fixture
def git_env(tmp_path):
    cfg = tmp_path / "gitconfig"
    cfg.write_text("[user]\n\tname = t\n\temail = t@example.invalid\n[core]\n\tautocrlf = false\n",
                   encoding="utf-8")
    env = dict(os.environ)
    env.update(GIT_CONFIG_GLOBAL=str(cfg), GIT_CONFIG_NOSYSTEM="1")
    return env


def test_two_branches_append_and_merge_without_conflict(tmp_path, git_env):
    repo = tmp_path / "repo"
    repo.mkdir()
    _git(repo, "init", "-q", "-b", "main", env=git_env)
    # the real attribute lines, not a copy of what we hope they say
    (repo / ".gitattributes").write_bytes((REPO_ROOT / ".gitattributes").read_bytes())
    path = repo / "qa" / "card-loop" / "attempts.jsonl"
    base = _attempt(run_id="base")
    AttemptLedger(path).append(base)
    _git(repo, "add", ".", env=git_env)
    _git(repo, "commit", "-q", "-m", "base", env=git_env)

    _git(repo, "switch", "-q", "-c", "run-a", env=git_env)
    run_a = [_attempt(run_id="run-a") for _ in range(3)]
    for a in run_a:
        AttemptLedger(path).append(a)
    _git(repo, "commit", "-q", "-am", "run a", env=git_env)

    _git(repo, "switch", "-q", "main", env=git_env)
    _git(repo, "switch", "-q", "-c", "run-b", env=git_env)
    run_b = [_attempt(run_id="run-b", family="codex") for _ in range(2)]
    for a in run_b:
        AttemptLedger(path).append(a)
    _git(repo, "commit", "-q", "-am", "run b", env=git_env)

    _git(repo, "merge", "-q", "--no-edit", "run-a", env=git_env)   # raises on conflict
    got, problems = load_attempts(path)
    assert problems == []
    assert {a.attempt_id for a in got} == {x.attempt_id for x in [base, *run_a, *run_b]}
    assert b"\r" not in path.read_bytes()


def test_read_rows_reports_line_numbers(tmp_path):
    p = tmp_path / "x.jsonl"
    append_row(p, {"a": 1})
    append_row(p, {"b": 2})
    rows, problems = read_rows(p)
    assert rows == [(1, {"a": 1}), (2, {"b": 2})] and problems == []
    with pytest.raises(TypeError):
        append_row(p, ["not", "a", "dict"])
