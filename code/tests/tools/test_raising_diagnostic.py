from collections import Counter

import pytest

from tools.cloud_search.raising_diagnostic import interval, paired_branches, requests, run_stage, write


def branch(action, status="complete", score=1):
    return dict(model=2, opponent=0, first_action=action,
        outcome=dict(status=status, score=score))


def test_incomplete_counterfactual_is_not_a_draw_or_unpaired_win():
    assert paired_branches({"branches": [branch(61), branch(62, "incomplete", None)]}) == {}
    assert paired_branches({"branches": [branch(61)]}) == {}
    assert paired_branches({"branches": [branch(61), branch(62, score=0)]}) == {(2, 0): {61: 1, 62: 0}}


def test_duplicate_branch_cannot_inflate_sample_size():
    with pytest.raises(ValueError, match="Duplicate"):
        paired_branches({"branches": [branch(61), branch(61), branch(62)]})


def test_paired_bootstrap_preserves_strata_and_zero_differences():
    result = interval([("A", 0)] * 4 + [("B", 0)] * 4)
    assert result == {"n": 8, "mean": 0.0, "ci95": [0.0, 0.0]}
    rows = [("A", 1), ("A", -1), ("B", 0), ("B", 1)]
    assert interval(rows) == interval(rows)
    assert interval(rows)["mean"] == 0.25
    assert interval([])["mean"] is None


def test_collection_balances_models_seats_and_list_variants(tmp_path):
    plan = dict(seed=1, actors=[0, 2], decks=[dict(name=name, cards=[])
        for name in ("A", "A", "B", "C", "C")])
    jobs = list(requests(tmp_path, plan, "collect"))
    assert len(jobs) == 12
    assert len({key for key, _ in jobs}) == 12
    for archetype in ("A", "B", "C"):
        rows = [request for _, request in jobs if request["archetype"] == archetype]
        assert Counter((r["source_model"], r["actor"]) for r in rows) == {
            (0, 0): 1, (0, 1): 1, (2, 0): 1, (2, 1): 1}
        if archetype != "B":
            assert len({r["deck_indices"][r["actor"]] for r in rows}) == 2


def test_singleton_strata_do_not_report_false_certainty():
    result = interval([("A", 1), ("B", 0), ("B", 0)])
    assert result["mean"] == pytest.approx(1 / 3)
    assert result["ci95"] is None
    assert "only one" in result["ci_note"]


def test_failure_sidecars_do_not_shift_frozen_probe_seeds(tmp_path):
    collect = tmp_path / "collect"
    collect.mkdir()
    write(collect / "00.json", dict(status="complete", node={"id": "00"}))
    write(collect / "01.json", dict(status="complete", node={"id": "01"}))
    before = list(requests(tmp_path, {"seed": 3}, "probe"))
    write(collect / "00.failure.json", dict(status="failed"))
    assert list(requests(tmp_path, {"seed": 3}, "probe")) == before


@pytest.mark.parametrize("status", ["failed", "running", "incomplete"])
def test_reentry_preserves_failed_or_partial_outputs_and_fails_closed(tmp_path, status, monkeypatch):
    from tools.cloud_search import raising_diagnostic as diagnostic

    write(tmp_path / "plan.json", dict(simulations=320, worlds=4, batch=4,
        exploration_noise=0, hashes={}))
    destination = tmp_path / "probe"
    destination.mkdir()
    target = destination / ("00.failure.json" if status == "failed" else "00.json")
    write(target, dict(status=status, evidence="preserve"))
    before = target.read_bytes()
    log = destination / "00.log"
    log.write_text("original failure details")
    monkeypatch.setattr(diagnostic, "requests", lambda *args: [("00", {})])
    monkeypatch.setattr(diagnostic.subprocess, "run", lambda *args, **kwargs: pytest.fail("must not rerun"))
    with pytest.raises(RuntimeError, match="preserved outputs"):
        run_stage(tmp_path, "probe", 1, None)
    assert target.read_bytes() == before
    assert log.read_text() == "original failure details"


def test_plan_cannot_misreport_native_search_budget(tmp_path):
    write(tmp_path / "plan.json", dict(simulations=640, worlds=4, batch=4,
        exploration_noise=0, hashes={}))
    with pytest.raises(ValueError, match="fixed at 320"):
        run_stage(tmp_path, "probe", 1, None)
