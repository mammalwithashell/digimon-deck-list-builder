"""Unit checks for the cap-randomization generation tooling (no native engine, no torch)."""
from __future__ import annotations

import json
from types import SimpleNamespace

import numpy as np
import pytest

from tools import cap_randomization_generation as gen
from tools import policy_head_to_head as h2h
from tools import sb3_from_onnx


def test_infer_architecture_reads_shapes_from_initializers():
    weights = {
        "features_extractor.embedding.weight": np.zeros((20000, 8), np.float32),
        "features_extractor.projection.0.weight": np.zeros((128, 13662), np.float32),
        "mlp_extractor_policy.0.weight": np.zeros((128, 128), np.float32),
        "mlp_extractor_policy.2.weight": np.zeros((64, 128), np.float32),
        "mlp_extractor_value.0.weight": np.zeros((128, 128), np.float32),
        "mlp_extractor_value.2.weight": np.zeros((128, 128), np.float32),
    }
    assert sb3_from_onnx.infer_architecture(weights) == {
        "vocab_size": 20000, "embedding_dim": 8, "features_dim": 128, "pi": [128, 64], "vf": [128, 128]}


def test_map_state_dict_aliases_shared_extractor_and_rejects_unknown_keys():
    torch = pytest.importorskip("torch")
    weights = {"features_extractor.embedding.weight": np.ones((2, 2), np.float32),
               "mlp_extractor_policy.0.bias": np.zeros(2, np.float32),
               "action_net.bias": np.zeros(3, np.float32)}
    target = ["features_extractor.embedding.weight", "pi_features_extractor.embedding.weight",
              "vf_features_extractor.embedding.weight", "mlp_extractor.policy_net.0.bias", "action_net.bias"]
    mapped = sb3_from_onnx.map_state_dict(weights, target)
    assert set(mapped) == set(target)
    assert torch.equal(mapped["vf_features_extractor.embedding.weight"], mapped["features_extractor.embedding.weight"])
    with pytest.raises(ValueError, match="unmapped"):
        sb3_from_onnx.map_state_dict({"mystery.weight": np.zeros(1, np.float32)}, [])
    with pytest.raises(ValueError, match="missing"):
        sb3_from_onnx.map_state_dict(weights, target + ["value_net.bias"])


def test_head_to_head_schedule_is_seat_balanced_per_block(tmp_path):
    decks = tmp_path / "decks.json"
    decks.write_text(json.dumps([
        {"name": "A:sha:slot0", "cards": ["a0"]}, {"name": "A:sha:slot1", "cards": ["a1"]},
        {"name": "B:sha:slot0", "cards": ["b0"]}, {"name": "C:sha:slot0", "cards": ["c0"]}]))
    pool = h2h.load_pool(decks)
    assert list(pool) == ["A", "B", "C"] and len(pool["A"]) == 2
    rows = h2h.schedule(pool, blocks_per_pair=1, seed=5)
    assert len(rows) == 6 * 4  # ordered pairs x four games
    for block in range(6):
        games = [r for r in rows if r["block"] == block]
        assert sorted(r["seed"] % 2 for r in games) == [0, 0, 1, 1]
        assert len({r["candidate_archetype"] for r in games}) == 2
        for r in games:
            assert r["candidate_deck"] != r["opponent_deck"]
    assert len({r["seed"] for r in rows}) == len(rows)


def test_block_interval_resamples_blocks_not_games():
    mean, (lo, hi) = h2h.block_interval([1, 1, 0, 0], [0, 0, 1, 1])
    assert mean == 0.5 and lo == 0.0 and hi == 1.0
    mean, (lo, hi) = h2h.block_interval([1, 0, 1, 0], [0, 0, 1, 1])
    assert (mean, lo, hi) == (0.5, 0.5, 0.5)


def test_plan_shares_cap_partitions_with_equal_compute_arm(tmp_path):
    args = SimpleNamespace(games=1000, partition=250, cap_cost_ratio=0.36, seed=1, sims=320, worlds=4,
                           eval_batch_size=4, decision_cap=1200, temperature_decisions=16, out=tmp_path,
                           arms=["full", "cap", "cap_eq", "phase", "phase_eq", "both_eq"])
    p = gen.plan(args)
    assert p["cap_eq_games"] == 3000  # ceil(1000/0.36 / 250) * 250
    assert p["arms"]["phase"]["flags"] == ["--reduced-sims", "64"]
    assert p["arms"]["phase_eq"]["games"] == 2500  # ceil(1000/0.44 / 250) * 250
    assert p["arms"]["phase_eq"]["generate_partitions"][0] == (1000, 250)
    assert p["arms"]["both_eq"]["games"] == 4250 and p["arms"]["both_eq"]["flags"][:2] == ["--reduced-sims", "64"]
    assert gen.shard_dirs(args, "phase_eq", p)[0] == tmp_path / "shards" / "phase" / "g00000-250"
    assert gen.shard_dirs(args, "phase_eq", p)[4] == tmp_path / "shards" / "phase_eq" / "g01000-250"
    with pytest.raises(SystemExit, match="unknown arm"):
        gen.plan(SimpleNamespace(**{**vars(args), "arms": ["gumbel"]}))
    assert p["arms"]["cap"]["generate_partitions"] == [(0, 250), (250, 250), (500, 250), (750, 250)]
    assert p["arms"]["cap_eq"]["generate_partitions"][0] == (1000, 250)
    assert len(p["arms"]["cap_eq"]["partitions"]) == 12
    dirs = gen.shard_dirs(args, "cap_eq", p)
    assert dirs[0] == tmp_path / "shards" / "cap" / "g00000-250"
    assert dirs[4] == tmp_path / "shards" / "cap_eq" / "g01000-250"
    assert gen.shard_dirs(args, "full", p)[0] == tmp_path / "shards" / "full" / "g00000-250"
