"""Unit checks for the search-budget diagnostic drivers (no native engine)."""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest

from tools import benchmark_search_budget, search_budget_experiment


def test_arrays_equal_compares_every_shard_array(tmp_path):
    left, right = tmp_path / "left", tmp_path / "right"
    for root in (left, right):
        (root / "shard-000").mkdir(parents=True)
        np.save(root / "shard-000" / "z.npy", np.array([1.0, -1.0], np.float32))
    np.save(left / "shard-000" / "policy_weight.npy", np.ones(2, np.float32))
    np.save(right / "shard-000" / "policy_weight.npy", np.array([1.0, 0.0], np.float32))
    checks = benchmark_search_budget.arrays_equal(left, right)
    assert checks == {"shard-000/policy_weight.npy": False, "shard-000/z.npy": True}
    # A missing array on the right is a failure, never silently skipped.
    np.save(left / "shard-000" / "mover.npy", np.zeros(2, np.uint8))
    assert benchmark_search_budget.arrays_equal(left, right)["shard-000/mover.npy"] is False


def test_benchmark_candidates_reduce_to_baseline_flags():
    c = benchmark_search_budget.CANDIDATES
    assert c["B-320"] == {"flags": []}
    assert "--reduced-sims" in c["C1-main320-other64"]["flags"]
    assert "--full-search-fraction" in c["C2-cap0.25-cheap64"]["flags"]
    # Budget overrides go through `sims`, never a duplicated --sims flag.
    assert c["G1-gumbel128-m16"]["sims"] == 128
    assert all("--sims" not in cand["flags"] for cand in c.values())
    for guard in benchmark_search_budget.GUARDS.values():
        assert guard["flags"], "every guard changes a flag that must not change the output"


def test_candidate_action_uses_the_improved_policy_when_recorded():
    # Gumbel rows: the completed-Q improved policy decides, not the visits
    # (sequential halving leaves visits concentrated on the exploratory
    # survivor) and not the survivor itself.
    rows = [{"action": 9, "survivor": 9, "visits": [[7, 1], [9, 5]], "policy": [[7, 0.7], [9, 0.3]]},
            {"action": 9, "survivor": 9, "visits": [[7, 2], [9, 4]], "policy": [[7, 0.6], [9, 0.4]]}]
    assert search_budget_experiment._candidate_action(rows) == 7
    puct = [{"action": 9, "visits": [[7, 1], [9, 5]], "policy": None},
            {"action": 9, "visits": [[7, 2], [9, 4]], "policy": None}]
    assert search_budget_experiment._candidate_action(puct) == 9


def test_prepare_binds_engine_and_parent_inputs(tmp_path):
    parent = tmp_path / "parent"
    parent.mkdir()
    model = tmp_path / "seed.onnx"
    cards = tmp_path / "cards.json"
    model.write_bytes(b"model")
    cards.write_text("{}")
    (parent / "corpus.json").write_text(json.dumps({"nodes": []}))
    (parent / "plan.json").write_text(json.dumps({
        "model": str(model), "cards_json": str(cards), "decks": [{"name": "a", "cards": []}],
        "references": [{"id": "s2560-w8", "simulations": 2560, "worlds": 8}],
        "search_seeds": [1, 2], "baseline": {"id": "s320-w4", "simulations": 320, "worlds": 4},
        "decision_cap": 1200, "regime": "open"}))
    engine = tmp_path / "search_quality.exe"
    engine.write_bytes(b"engine")
    out = tmp_path / "study"
    rc = search_budget_experiment.main([
        "prepare", "--parent-study", str(parent), "--engine", str(engine), "--out", str(out),
        "--candidates", json.dumps([{"id": "x", "simulations": 64, "worlds": 4, "root": "gumbel"}])])
    assert rc == 0
    plan = json.loads((out / "plan.json").read_text())
    assert plan["candidates"] == [{"id": "x", "simulations": 64, "worlds": 4, "root": "gumbel"}]
    assert plan["engine"] == str(engine.resolve())
    assert plan["baseline"]["id"] == "s320-w4"
    assert plan["corpus"] == str((parent / "corpus.json").resolve())
    assert "training_state" not in plan
    with pytest.raises(SystemExit):
        search_budget_experiment.main([
            "prepare", "--parent-study", str(parent), "--engine", str(engine), "--out", str(tmp_path / "dup"),
            "--candidates", json.dumps([{"id": "x", "simulations": 1, "worlds": 1}, {"id": "x", "simulations": 2, "worlds": 1}])])
