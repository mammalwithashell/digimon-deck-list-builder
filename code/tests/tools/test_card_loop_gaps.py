"""Gap lane (add-card-authoring-loop task 6.6, spec "Gaps are a ranked lane that
parks and unparks cards"): park, rank by core clauses blocked, close, unpark,
and orchestrator-only tracker writes."""
import json
import subprocess
from types import SimpleNamespace

import pytest

from tools.card_loop.gaps import (
    DSL_TRACKER,
    ENGINE_TRACKER,
    RunGapLane,
    clause_counts_from_plan,
    tracker_status,
)

NOW = "2026-10-06T12:00:00.000000Z"


def git(cwd, *args):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True,
                          text=True, encoding="utf-8").stdout.strip()


@pytest.fixture
def gitenv(tmp_path, monkeypatch):
    cfg = tmp_path / "gitconfig"
    cfg.write_text("[user]\n\tname = t\n\temail = t@example.invalid\n[core]\n\tautocrlf = false\n",
                   encoding="utf-8")
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(cfg))
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")


@pytest.fixture
def repo(tmp_path, gitenv):
    root = tmp_path / "repo"
    (root / "qa").mkdir(parents=True)
    (root / "docs").mkdir()
    (root / DSL_TRACKER).write_text(
        "# DSL Vocabulary Gaps Tracker\n\n"
        "## `select_hand` any zone  [G-DSL-OLD-OPEN] — OPEN\n\n- **Gap:** x\n\n"
        "## Trait substring  [G-DSL-OLD-DONE] — RESOLVED 2026-06-03\n\n- **Gap:** y\n\n"
        "## Status line form  [G-DSL-STATUS-FORM]\n\n- **Status: RESOLVED 2026-05-17** (Track H)\n",
        encoding="utf-8")
    (root / ENGINE_TRACKER).write_text(
        "# Rust Engine Gaps\n\n"
        "## TS findings (2026-10-04) — OPEN, logged not fixed\n\n"
        "### Counter window  [G-ENGINE-COUNTER] — confirmed in code\n\n- open still\n\n"
        "## G-ENGINE-LIVELOCK (BT20-100) — FIXED 2026-08-28\n\n- done\n",
        encoding="utf-8")
    git(root, "init", "-q", "-b", "card-loop/run1/run")
    git(root, "add", "-A")
    git(root, "commit", "-qm", "base")
    return root, git(root, "rev-parse", "HEAD")


PLAN = {
    "run_id": "run1",
    "work_set": {"pool": ["BT1-001", "BT1-002", "BT1-003", "BT1-004"],
                 "core": ["BT1-001", "BT1-002", "BT1-003"]},
    "exam": [
        {"card_id": "BT1-001", "clause_id": "BT1-001-effect1", "is_core": True},
        {"card_id": "BT1-001", "clause_id": "BT1-001-effect2", "is_core": True},
        {"card_id": "BT1-002", "clause_id": "BT1-002-effect1", "is_core": True},
        {"card_id": "BT1-004", "clause_id": "BT1-004-effect1", "is_core": False},
        {"card_id": "BT1-004", "clause_id": "BT1-004-effect2", "is_core": False},
        {"card_id": "BT1-004", "clause_id": "BT1-004-effect3", "is_core": False},
    ],
}


def lane(tmp_path, plan=PLAN, **kw):
    return RunGapLane(tmp_path / "run", plan, now=lambda: NOW, **kw)


# --- park / persist ------------------------------------------------------------------------


def test_park_persists_rows_and_merges_repeat_parks(tmp_path):
    gl = lane(tmp_path)
    gl.park("card:BT1-001", "G-A", is_core=True)
    gl.park("card:BT1-004", "G-A", is_core=False)
    gl.park("card:BT1-001", "G-A", is_core=True)          # idempotent

    doc = json.loads((tmp_path / "run" / "gaps.json").read_text(encoding="utf-8"))
    (row,) = doc["gaps"]
    assert row["gap_id"] == "G-A"
    assert row["items"] == ["card:BT1-001", "card:BT1-004"]
    assert row["core_items"] == ["card:BT1-001"]
    assert row["is_core"] is True and row["parked_at"] == NOW
    assert "branch" not in row

    # a fresh instance (resume) reads the same state
    assert lane(tmp_path).rows() == doc["gaps"]


def test_park_rejects_malformed_items(tmp_path):
    with pytest.raises(ValueError):
        lane(tmp_path).park("BT1-001", "G-A", is_core=True)
    with pytest.raises(ValueError):
        lane(tmp_path).park("card:BT1-001", "", is_core=True)


# --- ranking -------------------------------------------------------------------------------


def test_three_core_cards_outrank_one_support_card(tmp_path):
    gl = lane(tmp_path)
    gl.park("card:BT1-004", "G-SUPPORT", is_core=False)    # 3 clauses, but not core
    for c in ("BT1-001", "BT1-002", "BT1-003"):
        gl.park(f"card:{c}", "G-CORE", is_core=True)
    # BT1-001: 2 clauses, BT1-002: 1, BT1-003: no exam entries -> counts 1
    assert gl.ranked() == [("G-CORE", 4), ("G-SUPPORT", 0)]


def test_ranking_counts_clauses_and_breaks_ties_by_items_parked(tmp_path):
    gl = lane(tmp_path)
    gl.park("card:BT1-001", "G-TWO-CLAUSES", is_core=True)              # 2 core clauses
    gl.park("clause:BT1-002#effect#0", "G-TIE", is_core=True)           # 1
    gl.park("interaction:qa:Q100", "G-TIE", is_core=True)               # 1
    gl.park("card:BT1-004", "G-TIE", is_core=False)                     # 0 (support)
    gl.park("clause:BT1-003#effect#0", "G-LESS", is_core=True)          # 1
    gl.park("clause:BT1-003#effect#1", "G-LESS", is_core=True)          # 1
    assert gl.ranked() == [("G-TIE", 2), ("G-LESS", 2), ("G-TWO-CLAUSES", 2)]


def test_noted_branch_without_items_is_not_ranked(tmp_path):
    gl = lane(tmp_path)
    gl.note_branch("G-X", "card-loop/run1/engine-G-X", sha="abc")
    assert gl.ranked() == []
    assert gl.get("G-X")["branch"] == "card-loop/run1/engine-G-X"


def test_clause_counts_from_plan_shapes():
    assert clause_counts_from_plan(PLAN)["BT1-001"] == ["BT1-001-effect1", "BT1-001-effect2"]
    assert clause_counts_from_plan({"campaign": {"exam": PLAN["exam"]}})["BT1-004"] == [
        "BT1-004-effect1", "BT1-004-effect2", "BT1-004-effect3"]
    assert clause_counts_from_plan({"exams": {"BT1-009": 3}})["BT1-009"] == [
        "BT1-009#1", "BT1-009#2", "BT1-009#3"]
    assert clause_counts_from_plan({}) == {}


# --- unpark --------------------------------------------------------------------------------


def test_unpark_returns_items_and_removes_the_row(tmp_path):
    gl = lane(tmp_path)
    gl.park("card:BT1-001", "G-A", is_core=True)
    gl.park("card:BT1-002", "G-A", is_core=True)
    gl.park("card:BT1-003", "G-B", is_core=True)

    assert gl.unpark("G-A") == ["card:BT1-001", "card:BT1-002"]
    assert [r["gap_id"] for r in lane(tmp_path).rows()] == ["G-B"]
    assert gl.unpark("G-A") == []
    hist = json.loads((tmp_path / "run" / "gaps.json").read_text(encoding="utf-8"))["unparked"]
    assert hist[0]["gap_id"] == "G-A" and hist[0]["unparked_at"] == NOW


# --- closed --------------------------------------------------------------------------------


def ctx_for(root, base):
    return SimpleNamespace(repo=str(root), base_sha=base, run_id="run1")


def test_closed_when_the_engine_branch_is_merged_into_the_run_branch(tmp_path, repo):
    root, base = repo
    gl = lane(tmp_path, repo=root)
    gl.park("card:BT1-001", "G-ENG", is_core=True)
    branch = "card-loop/run1/engine-G-ENG"
    git(root, "branch", branch, base)
    gl.note_branch("G-ENG", branch)
    assert gl.closed(ctx_for(root, base)) == []          # no commit on the branch yet

    git(root, "checkout", "-q", branch)
    (root / "fix.rs").write_text("fn fixed() {}\n", encoding="utf-8")
    git(root, "add", "fix.rs")
    git(root, "commit", "-qm", "engine fix")
    git(root, "checkout", "-q", "card-loop/run1/run")
    assert gl.closed(ctx_for(root, base)) == []          # not merged yet

    git(root, "merge", "-q", "--no-ff", "-m", "human merge", branch)
    assert gl.closed(ctx_for(root, base)) == ["G-ENG"]


def test_closed_by_a_recorded_sha_after_the_branch_is_deleted(tmp_path, repo):
    root, base = repo
    gl = lane(tmp_path, repo=root)
    gl.park("card:BT1-001", "G-ENG", is_core=True)
    (root / "fix.rs").write_text("x\n", encoding="utf-8")
    git(root, "add", "fix.rs")
    git(root, "commit", "-qm", "fix landed")
    gl.note_branch("G-ENG", "card-loop/run1/engine-G-ENG", sha=git(root, "rev-parse", "HEAD"))
    assert gl.closed(ctx_for(root, base)) == ["G-ENG"]


def test_closed_by_the_tracker(tmp_path, repo):
    root, base = repo
    gl = lane(tmp_path, repo=root)
    for gap in ("G-DSL-OLD-OPEN", "G-DSL-OLD-DONE", "G-DSL-STATUS-FORM", "G-ENGINE-COUNTER",
                "G-ENGINE-LIVELOCK", "G-UNKNOWN"):
        gl.park("card:BT1-001", gap, is_core=True)
    assert gl.closed(ctx_for(root, base)) == ["G-DSL-OLD-DONE", "G-DSL-STATUS-FORM",
                                              "G-ENGINE-LIVELOCK"]


def test_tracker_status():
    text = ("## A  [G-A] — OPEN\n\n## B  [G-B] — **RESOLVED 2026-10-04**\n\n"
            "## findings — OPEN, logged not fixed\n\n### C  [G-C] — suspected\n\n"
            "> **Blockquote form — RESOLVED 2026-06-03**\n> long text G-D.\n\n"
            "### ✅ FIXED (2026-05-30) — thing [G-E]\n")
    assert tracker_status(text, "G-A") == "open"
    assert tracker_status(text, "G-B") == "closed"
    assert tracker_status(text, "G-C") == "open"
    assert tracker_status(text, "G-D") == "closed"
    assert tracker_status(text, "G-E") == "closed"
    assert tracker_status(text, "G-Z") is None
    assert tracker_status("## x [G-AB] — RESOLVED\n", "G-A") is None     # token, not prefix


# --- tracker writes (orchestrator only) ----------------------------------------------------


def test_record_gap_appends_a_well_formed_entry_once_and_commits(tmp_path, repo):
    root, base = repo
    gl = lane(tmp_path, repo=root)
    gl.park("card:BT1-001", "G-DSL-NEW-VERB", is_core=True)
    gap = {"id": "G-DSL-NEW-VERB", "summary": "`play_self_from_hand` with a cost delta"}

    assert gl.record_gap(gap, "dsl", attempt_id="20261006T120000Z-run1-ab12", family="claude",
                         model="sonnet") is True
    text = (root / DSL_TRACKER).read_text(encoding="utf-8")
    entry = text.split("\n## ")[-1]
    assert entry.startswith("`play_self_from_hand` with a cost delta  [G-DSL-NEW-VERB] — OPEN "
                            "(2026-10-06, card-loop run `run1`)")
    assert "<!-- card-loop-gap:G-DSL-NEW-VERB -->" in entry
    assert "- **Blocks:** card:BT1-001 (1 core)" in entry
    assert tracker_status(text, "G-DSL-NEW-VERB") == "open"
    msg = git(root, "log", "-1", "--format=%B")
    assert "Loop-Attempt: 20261006T120000Z-run1-ab12" in msg and "Loop-Model: claude/sonnet" in msg
    assert git(root, "status", "--porcelain") == ""

    assert gl.record_gap(gap, "dsl") is False                          # idempotent
    assert gl.record_gap({"id": "G-DSL-OLD-OPEN", "summary": "x"}, "dsl") is False   # already there
    assert (root / DSL_TRACKER).read_text(encoding="utf-8") == text


def test_record_gap_engine_tracker_and_bad_kind(tmp_path, repo):
    root, base = repo
    gl = lane(tmp_path, repo=root)
    assert gl.record_gap({"id": "G-ENGINE-NEW", "summary": "trigger order"}, "engine") is True
    assert "[G-ENGINE-NEW] — OPEN" in (root / ENGINE_TRACKER).read_text(encoding="utf-8")
    assert "Loop-Run: run1" in git(root, "log", "-1", "--format=%B")
    with pytest.raises(ValueError):
        gl.record_gap({"id": "G-X", "summary": "y"}, "card")


def test_record_gap_refuses_to_commit_on_main(tmp_path, gitenv):
    root = tmp_path / "repo"
    (root / "qa").mkdir(parents=True)
    (root / DSL_TRACKER).write_text("# DSL\n", encoding="utf-8")
    git(root, "init", "-q", "-b", "main")
    git(root, "add", "-A")
    git(root, "commit", "-qm", "base")
    gl = lane(tmp_path, repo=root)
    assert gl.record_gap({"id": "G-DSL-Z", "summary": "z"}, "dsl") is True
    assert git(root, "log", "--oneline").count("\n") == 0              # still one commit
    assert git(root, "status", "--porcelain") == f"M {DSL_TRACKER}"
