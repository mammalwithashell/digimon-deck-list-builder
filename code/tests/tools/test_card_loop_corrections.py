"""Correction events (tasks 4.3): detectors per spec scenario, writer, tolerant reader."""
import json
import subprocess

import pytest

from tools.card_loop import corrections as cx
from tools.card_loop._jsonl import REPO_ROOT
from tools.card_loop.corrections import (
    KINDS,
    Correction,
    CorrectionLog,
    detect_escalation_resolution,
    detect_late_edits,
    detect_overturned_verdict,
    detect_unexercised_clauses,
    family_disagreement,
    load_corrections,
    touched_old_lines,
)
from tools.card_loop.provenance import blame_attempts, loop_commit_message

TS = "2026-10-04T12:00:00.000000Z"


def test_kind_table_matches_design_d11():
    immediate = {k for k, v in KINDS.items() if v == "immediate"}
    assert immediate == {"review_reject", "gate_fail", "schema_invalid", "family_disagreement"}
    assert set(KINDS) - immediate == {"later_edit", "verdict_overturned",
                                      "escalation_resolved_against", "clause_not_exercised",
                                      "fix_reverted", "false_alarm"}
    for k in KINDS:
        assert k in cx.__doc__


# --- immediate ----------------------------------------------------------------------------

def test_review_reject_and_gate_fail_are_immediate():
    r = cx.review_reject("A-impl", reviewer_attempt="A-rev", stage="implement",
                         item="card:BT7-056", detail="misses the decline", ts=TS)
    assert (r.latency, r.by, r.stage) == ("immediate", ("attempt", "A-rev"), "implement")
    g = cx.gate_fail("A-impl", gate="cards_behavioral", stage="implement", ts=TS)
    assert (g.latency, g.by) == ("immediate", ("gate", "cards_behavioral"))
    s = cx.schema_invalid("A-x", stage="triage", ts=TS)
    assert (s.kind, s.by) == ("schema_invalid", ("gate", "schema"))


def test_family_disagreement_corrects_each_by_the_other():
    ev = family_disagreement(("T-claude", "dcgo_quirk"), ("T-codex", "ours_wrong"),
                             stage="triage", item="clause:BT7-056#effect#0", ts=TS)
    assert [(e.corrected_attempt, e.by_attempt, e.latency) for e in ev] == [
        ("T-claude", "T-codex", "immediate"), ("T-codex", "T-claude", "immediate")]
    assert family_disagreement(("a", "dcgo_quirk"), ("b", "dcgo_quirk"), stage="triage") == []
    assert family_disagreement(("a", "dcgo_quirk"), ("b", "undetermined"), stage="triage") == []


# --- spec: "Late correction of a triage call" ---------------------------------------------

def test_overturned_quirk_corrects_both_agreeing_triage_attempts():
    calls = {"T-claude": "dcgo_quirk", "T-codex": "dcgo_quirk"}
    ev = detect_overturned_verdict(calls, overturned="dcgo_quirk", new_verdict="ours_wrong",
                                   stage="triage", by_human="james", item="clause:X#effect#0", ts=TS)
    assert sorted(e.corrected_attempt for e in ev) == ["T-claude", "T-codex"]
    assert all(e.latency == "late" and e.kind == "verdict_overturned" and e.by_human == "james"
               for e in ev)
    assert ev[0].detail == "dcgo_quirk -> ours_wrong"


def test_overturn_spares_the_attempt_that_called_it_right():
    calls = {"T1": "dcgo_quirk", "T2": "ours_wrong", "T3": "dcgo_quirk"}
    ev = detect_overturned_verdict(calls, overturned="dcgo_quirk", new_verdict="ours_wrong",
                                   attempt_stages={"T1": "triage", "T3": "classify_qa"},
                                   by_attempt="F-fix", ts=TS)
    assert [(e.corrected_attempt, e.stage) for e in ev] == [("T1", "triage"), ("T3", "classify_qa")]
    assert detect_overturned_verdict(calls, overturned="x", new_verdict="x", stage="triage",
                                     by_human="h") == []


# --- spec: "Confirmed line that never exercised its clause" -------------------------------

def test_unexercised_confirmed_line_corrects_its_authoring_attempt():
    confirmed = {"BT7-056#effect#0": "A-auth1", "BT7-056#effect#1": "A-auth2",
                 "BT7-056#inherited#0": None}
    exercised = {"BT7-056#effect#0": False, "BT7-056#effect#1": True,
                 "BT7-056#inherited#0": False}
    ev = detect_unexercised_clauses(confirmed, exercised, by_attempt="I-probe", ts=TS)
    assert len(ev) == 1
    e = ev[0]
    assert (e.corrected_attempt, e.by_attempt, e.kind, e.latency, e.stage, e.item) == (
        "A-auth1", "I-probe", "clause_not_exercised", "late", "author_clause",
        "clause:BT7-056#effect#0")
    # not measured is not a correction
    assert detect_unexercised_clauses({"c": "A"}, {}, by_attempt="I") == []


def test_escalation_resolution_corrects_the_losing_calls_only():
    calls = {"T-claude": "dcgo_quirk", "T-codex": "ours_wrong", "T-x": "undetermined"}
    ev = detect_escalation_resolution(calls, resolution="ours_wrong", stage="triage", ts=TS)
    assert [(e.corrected_attempt, e.by_human, e.latency) for e in ev] == [
        ("T-claude", "human", "late")]


def test_fix_reverted_and_false_alarm():
    f = cx.fix_reverted("F1", reason="rejected_branch", by_human="james", ts=TS)
    assert (f.kind, f.latency, f.stage, f.detail) == ("fix_reverted", "late", "fix_card",
                                                      "rejected_branch")
    with pytest.raises(ValueError):
        cx.fix_reverted("F1", reason="vibes", by_human="james")
    a = cx.false_alarm("I1", by_human="james", detail="scenario forgot the memory gauge", ts=TS)
    assert (a.kind, a.latency, a.stage) == ("false_alarm", "late", "author_interaction")


def test_validation_rules():
    with pytest.raises(ValueError):    # nobody corrected it
        Correction(ts=TS, corrected_attempt="A", kind="later_edit", latency="late").validate()
    with pytest.raises(ValueError):    # two correctors
        Correction(ts=TS, corrected_attempt="A", kind="later_edit", latency="late",
                   by_attempt="B", by_human="h").validate()
    with pytest.raises(ValueError):    # latency contradicts kind
        Correction(ts=TS, corrected_attempt="A", kind="review_reject", latency="late",
                   by_attempt="B").validate()
    with pytest.raises(ValueError):    # self-correction
        cx.review_reject("A", reviewer_attempt="A", stage="implement")


# --- late edits: pure detector + diff parser + an end-to-end pass over real git -----------

def test_detect_late_edits_groups_by_owner_and_skips_self_and_humans():
    blame = {3: "A-impl", 4: "A-impl", 5: "A-impl", 9: "A-impl", 10: None, 11: "F-fix",
             12: "A-other"}
    ev = detect_late_edits(blame, path="cards/bt7/BT7-056.yaml", by_attempt="F-fix",
                           attempt_stages={"A-impl": "implement"}, ts=TS)
    assert [(e.corrected_attempt, e.stage, e.detail) for e in ev] == [
        ("A-impl", "implement", "cards/bt7/BT7-056.yaml: lines 3-5,9"),
        ("A-other", None, "cards/bt7/BT7-056.yaml: lines 12")]
    assert all(e.kind == "later_edit" and e.latency == "late" for e in ev)


DIFF = """diff --git a/cards/x.yaml b/cards/x.yaml
index 1111111..2222222 100644
--- a/cards/x.yaml
+++ b/cards/x.yaml
@@ -2,4 +2,4 @@ id: X
 name: X
--- a yaml document separator deleted
-  - draw: 1
+  - draw: 2
+  - trash: 1
 tail
@@ -10 +11 @@
-last
+LAST
diff --git a/new.yaml b/new.yaml
new file mode 100644
--- /dev/null
+++ b/new.yaml
@@ -0,0 +1,2 @@
+a
+b
"""


def test_touched_old_lines_counts_hunks_exactly():
    # line 3's content starts with "-- " — it is a deleted line, not a file header
    assert touched_old_lines(DIFF) == {"cards/x.yaml": [3, 4, 10]}
    assert touched_old_lines("") == {}


def test_late_edit_end_to_end_through_git(tmp_path, monkeypatch):
    cfg = tmp_path / "gitconfig"
    cfg.write_text("[user]\n\tname = t\n\temail = t@example.invalid\n[core]\n\tautocrlf = false\n",
                   encoding="utf-8")
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(cfg))
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")
    repo = tmp_path / "r"
    repo.mkdir()

    def git(*a):
        return subprocess.run(["git", *a], cwd=repo, check=True, capture_output=True,
                              text=True, encoding="utf-8").stdout

    def commit(text, msg):
        (repo / "c.yaml").write_bytes(text.encode("utf-8"))
        git("add", "c.yaml")
        (tmp_path / "m").write_bytes(msg.encode("utf-8"))
        git("commit", "-q", "-F", str(tmp_path / "m"))
        return git("rev-parse", "HEAD").strip()

    git("init", "-q", "-b", "main")
    before = commit("id: X\neffects:\n  - draw: 1\n  - trash: 1\n",
                    loop_commit_message("impl", None, attempt_id="A-impl", family="claude",
                                        model="sonnet"))
    after = commit("id: X\neffects:\n  - draw: 2\n  - trash: 1\nextra: added\n",
                   loop_commit_message("fix", None, attempt_id="F-fix", family="codex",
                                       model=None))
    touched = touched_old_lines(git("diff", before, after))
    assert touched == {"c.yaml": [3]}          # the appended line touches nothing old
    blame = blame_attempts(repo, "c.yaml", touched["c.yaml"], rev=before)
    ev = detect_late_edits(blame, path="c.yaml", by_attempt="F-fix", ts=TS)
    assert [(e.corrected_attempt, e.by_attempt, e.detail) for e in ev] == [
        ("A-impl", "F-fix", "c.yaml: lines 3")]


# --- writer / reader ----------------------------------------------------------------------

def test_log_round_trip_and_tolerant_reader(tmp_path):
    p = tmp_path / "corrections.jsonl"
    log = CorrectionLog(p)
    events = family_disagreement(("a", "x"), ("b", "y"), stage="triage", ts=TS)
    log.extend(events)
    log.append(cx.gate_fail("c", gate="sim", stage="author_clause", ts=TS))
    with open(p, "ab") as f:
        f.write(b"garbage\n")
        f.write(json.dumps({"ts": TS, "corrected_attempt": "d", "kind": "future_kind",
                            "latency": "eventually", "by_human": "h", "new": 1}).encode() + b"\n")
        f.write(json.dumps({"kind": "gate_fail"}).encode() + b"\n")
    got, problems = load_corrections(p)
    assert [c.corrected_attempt for c in got] == ["a", "b", "c", "d"]
    assert got[:3] == [*events, got[2]] and got[3].extra == {"new": 1}
    assert sorted(pr.kind for pr in problems) == ["bad_value", "malformed_json", "missing_keys"]
    row = json.loads(p.read_text(encoding="utf-8").splitlines()[2])
    assert row == {"ts": TS, "corrected_attempt": "c", "by_gate": "sim", "kind": "gate_fail",
                   "latency": "immediate", "stage": "author_clause", "detail": ""}


def test_extend_validates_everything_before_writing(tmp_path):
    p = tmp_path / "c.jsonl"
    good = cx.gate_fail("c", gate="sim", stage="author_clause", ts=TS)
    bad = Correction(ts=TS, corrected_attempt="x", kind="nope", latency="late", by_human="h")
    with pytest.raises(ValueError):
        CorrectionLog(p).extend([good, bad])
    assert not p.exists()


def test_default_path_is_beside_the_attempt_ledger():
    assert CorrectionLog().path == REPO_ROOT / "qa" / "card-loop" / "corrections.jsonl"
