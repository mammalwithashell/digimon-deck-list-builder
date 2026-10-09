"""Card stages: IMPLEMENTING (DSL-first) and REVIEW (the other family)."""
from __future__ import annotations

import pytest

from tools.card_loop.driver_contracts import ItemRecord, check_transition
from tools.card_loop.stages.base import StageDeferred
from tools.card_loop.stages.implement import ImplementExecutor
from tools.card_loop.stages.review import ReviewExecutor
from tools.card_loop.stages.testing import failed, make_ctx, ok
from tools.card_loop.workers.fake import FakeWorker

IMPL_OK = {"files": ["code/digimon-engine/cards/bt7/BT7-056.yaml"], "tests": ["bt7::bt7_056::on_play"],
           "test_result_lines": ["test result: ok. 3 passed; 0 failed"], "gaps": [], "notes": ""}
ARTS = {"diff": "D:/a/worktree.diff", "manifest": "D:/a/manifest.json"}


@pytest.fixture
def repo(tmp_path):
    (tmp_path / "data" / "card_bundles").mkdir(parents=True)
    (tmp_path / "data" / "card_bundles" / "BT7-056.md").write_text("# Dorumon  (BT7-056)\n", encoding="utf-8")
    cards = tmp_path / "code" / "digimon-engine" / "cards" / "bt7"
    cards.mkdir(parents=True)
    for cid in ("BT7-050", "BT7-055", "BT7-060", "BT7-090"):
        (cards / f"{cid}.yaml").write_text("id: x\n", encoding="utf-8")
    return tmp_path


def _workers(claude=(), codex=()):
    return {"claude": FakeWorker(list(claude), family="claude"),
            "codex": FakeWorker(list(codex), family="codex")}


def _card(state="IMPLEMENTING", **data):
    return ItemRecord(item="card:BT7-056", state=state, data=data)


# ---------------------------------------------------------------- IMPLEMENTING


def test_implement_goes_to_review_with_a_non_engine_merge_request(repo):
    workers = _workers(claude=[ok(IMPL_OK, artifacts=ARTS)])
    ctx = make_ctx(repo, workers=workers)
    item = _card()
    out = ImplementExecutor().run(ctx, item)
    check_transition("card", item.state, out.next_state)
    assert out.next_state == "REVIEW"
    mr = out.merge_request
    assert mr is not None and mr.engine is False and mr.family == "claude"
    assert mr.attempt_id == "att-0001" and mr.artifacts == ARTS and mr.model == "sonnet"
    assert out.data["implementer_family"] == "claude"
    assert out.data["implement_attempt"] == "att-0001"
    (a,) = out.attempts
    assert (a.stage, a.family, a.outcome, a.assignment, a.prompt_version) == (
        "implement", "claude", "accepted", "routed", "2")
    pkt = workers["claude"].received[0]
    assert "# produced_by: att-0001" in pkt.prompt
    assert "Dorumon (BT7-056)" in pkt.prompt
    assert "data/card_bundles/BT7-056.md" in pkt.references
    # the two nearest specs of the set are the imitation examples
    assert "BT7-055.yaml" in pkt.prompt and "BT7-060.yaml" in pkt.prompt and "BT7-090" not in pkt.prompt
    assert pkt.worktree == str(repo)


def test_existing_spec_is_audited_not_rewritten(repo):
    (repo / "code/digimon-engine/cards/bt7/BT7-056.yaml").write_text("id: BT7-056\n", encoding="utf-8")
    workers = _workers(claude=[ok(IMPL_OK)])
    ImplementExecutor().run(make_ctx(repo, workers=workers), _card())
    assert "AUDIT" in workers["claude"].received[0].prompt


def test_gaps_park_the_card_on_the_first_gap_without_a_merge(repo):
    res = dict(IMPL_OK, gaps=[{"kind": "dsl", "id": "G-DSL-REVEAL-BUCKET", "summary": "..."},
                              {"kind": "engine", "id": "G-ENG-X", "summary": "..."}])
    out = ImplementExecutor().run(make_ctx(repo, workers=_workers(claude=[ok(res)])), _card())
    check_transition("card", "IMPLEMENTING", out.next_state)
    assert out.next_state == "PARKED"
    assert out.data["gap_id"] == "G-DSL-REVEAL-BUCKET"
    assert [g["id"] for g in out.data["gaps"]] == ["G-DSL-REVEAL-BUCKET", "G-ENG-X"]
    assert out.merge_request is None
    assert out.data["implementer_family"] == "claude"


def test_review_directives_from_a_rejection_reach_the_next_attempt(repo):
    workers = _workers(claude=[ok(IMPL_OK)])
    item = _card(review_directives=[{"path": "code/x.rs", "directive": "test the decline path"}])
    out = ImplementExecutor().run(make_ctx(repo, workers=workers), item)
    assert "test the decline path" in workers["claude"].received[0].prompt
    assert out.data["review_directives"] is None


def test_schema_invalid_twice_defers_with_a_correction(repo):
    bad = ok({"files": "not-a-list"})
    workers = _workers(claude=[bad, bad])
    with pytest.raises(StageDeferred) as e:
        ImplementExecutor().run(make_ctx(repo, workers=workers), _card())
    assert workers["claude"].calls == 2
    assert "previous result was rejected" in workers["claude"].received[1].prompt
    o = e.value.outcome
    assert o.next_state == "IMPLEMENTING"
    (a,) = o.attempts
    assert a.outcome == "schema_invalid"
    (c,) = o.corrections
    assert (c.kind, c.corrected_attempt, c.by_gate) == ("schema_invalid", "att-0001", "schema")


def test_quota_exhaustion_reroutes_to_the_other_family(repo):
    workers = _workers(claude=[failed("quota_exhausted", "credit balance is too low")],
                       codex=[ok(IMPL_OK)])
    ctx = make_ctx(repo, workers=workers)
    out = ImplementExecutor().run(ctx, _card())
    assert out.next_state == "REVIEW"
    assert [a.family for a in out.attempts] == ["claude", "codex"]
    assert out.attempts[0].outcome == "quota_exhausted"
    assert out.data["implementer_family"] == "codex"
    assert not ctx.health.is_available("claude")


def test_no_family_left_defers(repo):
    ctx = make_ctx(repo, workers=_workers())
    ctx.health.disable("claude", "quota")
    ctx.health.disable("codex", "quota")
    with pytest.raises(StageDeferred):
        ImplementExecutor().run(ctx, _card())


# ---------------------------------------------------------------- REVIEW


def _review_item(implementer="claude"):
    return _card("REVIEW", implementer_family=implementer, implement_attempt="att-impl",
                 implement_result=IMPL_OK, artifacts=ARTS, history=["att-impl"])


def test_review_goes_to_the_other_family_than_the_implementer(repo):
    workers = _workers(claude=[ok({"verdict": "accept", "directives": [], "summary": "fine"})])
    out = ReviewExecutor().run(make_ctx(repo, workers=workers), _review_item(implementer="codex"))
    assert workers["claude"].calls == 1 and workers["codex"].calls == 0
    assert out.attempts[0].assignment == "forced"


def test_review_accept_implements_the_card(repo):
    workers = _workers(codex=[ok({"verdict": "accept", "directives": [], "summary": "fine"})])
    out = ReviewExecutor().run(make_ctx(repo, workers=workers), _review_item())
    check_transition("card", "REVIEW", out.next_state)
    assert out.next_state == "IMPLEMENTED" and out.adjudicated
    (a,) = out.attempts
    assert (a.stage, a.family, a.outcome, a.parent_attempt) == ("review", "codex", "accepted", "att-impl")
    assert not out.corrections
    pkt = workers["codex"].received[0]
    assert ARTS["diff"] in pkt.prompt and "att-impl" in pkt.prompt
    assert "test result: ok. 3 passed" in pkt.prompt


def test_review_reject_returns_to_implementing_with_a_correction(repo):
    directives = [{"path": "code/digimon-engine/cards/bt7/BT7-056.yaml", "directive": "add the decline"}]
    workers = _workers(codex=[ok({"verdict": "reject", "directives": directives, "summary": "no decline"})])
    out = ReviewExecutor().run(make_ctx(repo, workers=workers), _review_item())
    check_transition("card", "REVIEW", out.next_state)
    assert out.next_state == "IMPLEMENTING"
    assert out.data["review_directives"] == directives
    (c,) = out.corrections
    assert (c.kind, c.corrected_attempt, c.by_attempt, c.stage) == (
        "review_reject", "att-impl", "att-0001", "implement")
    assert out.attempts[0].outcome == "accepted"


def test_review_defers_when_only_the_implementer_family_is_left(repo):
    ctx = make_ctx(repo, workers=_workers())
    ctx.health.disable("codex", "quota")
    with pytest.raises(StageDeferred, match="codex"):
        ReviewExecutor().run(ctx, _review_item(implementer="claude"))
