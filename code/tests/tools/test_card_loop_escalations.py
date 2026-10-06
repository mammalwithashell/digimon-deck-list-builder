"""The human escalation queue (openspec add-card-authoring-loop, design D5, task 6.4)."""
from __future__ import annotations

import json

from tools.card_loop import escalations as esc_mod
from tools.card_loop.contracts import Usage
from tools.card_loop.driver_contracts import Escalation
from tools.card_loop.ledger import Attempt

ITEM = "clause:BT7-056#effect#0"


def _attempt(aid, stage, family, outcome="accepted", cost=0.25):
    return Attempt(attempt_id=aid, run_id="r1", ts="2026-10-05T00:00:00Z", stage=stage,
                   item=ITEM, family=family, model="m-" + family, effort=None,
                   prompt_version="v1", assignment="routed", outcome=outcome,
                   usage=Usage(cost_usd=cost))


def _disagreement():
    return Escalation(
        item=ITEM,
        reason="families disagree: claude dcgo_quirk vs codex ours_wrong",
        arguments=(
            {"family": "claude", "attempt_id": "A2", "call": "dcgo_quirk",
             "citation": "general_rule.pdf 15-1-2", "reasoning": "DCGO adds per condition."},
            {"family": "codex", "attempt_id": "A3", "call": "ours_wrong",
             "citation": None, "reasoning": "Ours never offers the decline."},
        ),
        history=("A1", "A2", "A3"),
    )


def test_escalation_file_carries_both_arguments_history_and_how_to_resolve(tmp_path):
    attempts = {a.attempt_id: a for a in (
        _attempt("A1", "author_clause", "claude"),
        _attempt("A2", "triage", "claude"),
        _attempt("A3", "triage", "codex", outcome="escalated", cost=None),
    )}
    path = esc_mod.write_escalation(tmp_path, _disagreement(), run_id="r1",
                                    ts="2026-10-05T01:02:03Z", attempts=attempts,
                                    state_before="TERMINATION_CHECK", item_data={"scenario": "x.yaml"})
    assert path.parent == tmp_path and path.suffix == ".md"
    raw = path.read_bytes()
    assert b"\r\n" not in raw, "escalation files are LF"
    text = raw.decode("utf-8")

    assert text.startswith("---\n")
    assert f"item: {ITEM}\n" in text
    assert "run_id: r1" in text and "state_before: TERMINATION_CHECK" in text
    assert "families disagree" in text
    assert "not adjudicated" in text.lower()

    # both families' arguments, with the citation or its absence called out
    assert "claude" in text and "dcgo_quirk" in text and "general_rule.pdf 15-1-2" in text
    assert "DCGO adds per condition." in text
    assert "codex" in text and "ours_wrong" in text and "Ours never offers the decline." in text
    assert "no citation" in text.lower()

    # attempt history, oldest first, with details from the ledger
    hist = text.index("## Attempt history")
    a1, a2, a3 = (text.index(x, hist) for x in ("A1", "A2", "A3"))
    assert a1 < a2 < a3
    assert "author_clause" in text[hist:] and "escalated" in text[hist:]

    assert "## How to resolve" in text
    assert "verdict-triage" in text, "names the command that records a triage call"
    assert '"scenario": "x.yaml"' in text


def test_every_escalation_appends_an_index_row(tmp_path):
    esc_mod.write_escalation(tmp_path, _disagreement(), run_id="r1", ts="t1")
    other = Escalation(item="card:BT7-056", reason="attempt cap: implement 3/3", history=("B1",))
    esc_mod.write_escalation(tmp_path, other, run_id="r1", ts="t2")
    rows = [json.loads(line) for line in
            (tmp_path / "index.jsonl").read_text(encoding="utf-8").splitlines()]
    assert [r["item"] for r in rows] == [ITEM, "card:BT7-056"]
    assert rows[0]["run_id"] == "r1" and rows[0]["ts"] == "t1"
    assert rows[0]["families"] == ["claude", "codex"]
    assert rows[1]["reason"].startswith("attempt cap")
    assert rows[1]["history"] == ["B1"]
    assert rows[0]["path"].endswith(".md")


def test_a_driver_escalation_without_arguments_says_so(tmp_path):
    e = Escalation(item="card:BT7-056", reason="attempt cap: implement 3/3", history=("B1", "B2"))
    text = esc_mod.write_escalation(tmp_path, e, run_id="r1", ts="t").read_text(encoding="utf-8")
    assert "No model arguments" in text
    assert "B1" in text and "B2" in text


def test_load_escalated_items_reads_open_files_only(tmp_path):
    p1 = esc_mod.write_escalation(tmp_path, _disagreement(), run_id="r1", ts="t")
    esc_mod.write_escalation(tmp_path, Escalation(item="card:BT7-056", reason="cap"),
                             run_id="r1", ts="t")
    (tmp_path / "README.md").write_text("# not an escalation\n", encoding="utf-8")
    assert set(esc_mod.load_escalated_items(tmp_path)) == {ITEM, "card:BT7-056"}

    p1.unlink()   # a human resolved it
    assert set(esc_mod.load_escalated_items(tmp_path)) == {"card:BT7-056"}
    assert esc_mod.load_escalated_items(tmp_path / "missing") == {}


def test_slugs_are_filesystem_safe_and_distinct():
    ids = [ITEM, "clause:BT7-056#effect#1", "interaction:qa:Q1601",
           "interaction:probe:BT7-056#effect#0:scope:neg", "card:BT7-056",
           "interaction:probe:" + "X" * 300]
    slugs = [esc_mod.item_slug(i) for i in ids]
    assert len(set(slugs)) == len(slugs)
    for s in slugs:
        assert s and len(s) <= 120
        assert all(c.isalnum() or c in "._-" for c in s), s
