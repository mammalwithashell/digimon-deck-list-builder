"""OpponentPool manifest schema, sampling, and training integration."""

from __future__ import annotations

import json

from digimon_gym.agents.opponent_pool import OpponentEntry, OpponentPool


def test_load_empty_pool(tmp_path):
    manifest = tmp_path / "pool.json"
    manifest.write_text(json.dumps({"version": 1, "entries": []}))
    pool = OpponentPool.load(manifest)
    assert pool.size == 0


def test_register_and_sample(tmp_path):
    manifest = tmp_path / "pool.json"
    manifest.write_text(json.dumps({"version": 1, "entries": []}))
    pool = OpponentPool.load(manifest)
    pool.register(
        OpponentEntry(
            name="gen0",
            weights_path="models/gen0/final.zip",
            algorithm="mlp",
            win_rate_vs_pool=0.5,
            games_played=0,
        )
    )
    pool.save()

    pool2 = OpponentPool.load(manifest)
    assert pool2.size == 1
    assert pool2.sample(rng_seed=0).name == "gen0"


def test_pfsp_sampling_prefers_low_winrate(tmp_path):
    manifest = tmp_path / "pool.json"
    manifest.write_text(json.dumps({"version": 1, "entries": []}))
    pool = OpponentPool.load(manifest)
    pool.register(
        OpponentEntry("strong", "a.zip", "mlp", win_rate_vs_pool=0.9, games_played=100)
    )
    pool.register(
        OpponentEntry("weak", "b.zip", "mlp", win_rate_vs_pool=0.1, games_played=100)
    )

    counts = {"strong": 0, "weak": 0}
    for seed in range(1000):
        counts[pool.sample(rng_seed=seed, mode="pfsp", power=2.0).name] += 1
    assert counts["weak"] > counts["strong"]


def test_record_match_updates_winrate(tmp_path):
    manifest = tmp_path / "pool.json"
    manifest.write_text(json.dumps({"version": 1, "entries": []}))
    pool = OpponentPool.load(manifest)
    pool.register(
        OpponentEntry("gen0", "x.zip", "mlp", win_rate_vs_pool=0.5, games_played=0)
    )
    pool.record_match("gen0", agent_won=True)
    pool.record_match("gen0", agent_won=False)
    pool.record_match("gen0", agent_won=True)
    entry = pool.get("gen0")
    assert entry.games_played == 3
    assert abs(entry.win_rate_vs_pool - (1 / 3)) < 1e-6


def test_pool_opponent_fn_sampling(tmp_path):
    from sb3_contrib import MaskablePPO
    from sb3_contrib.common.wrappers import ActionMasker

    from digimon_gym.agents.pilot_training import make_pool_opponent_fn
    from digimon_gym.digimon_gym import ACTION_SPACE_SIZE, DigimonEnv

    env = ActionMasker(DigimonEnv(), lambda e: e.action_mask())
    model = MaskablePPO("MlpPolicy", env, n_steps=64, batch_size=32, verbose=0)
    model.learn(total_timesteps=64)
    weights = tmp_path / "tiny.zip"
    model.save(str(weights))

    manifest = tmp_path / "pool.json"
    manifest.write_text(json.dumps({"version": 1, "entries": []}))
    pool = OpponentPool.load(manifest)
    pool.register(
        OpponentEntry(
            name="tiny",
            weights_path=str(weights),
            algorithm="mlp",
            win_rate_vs_pool=0.5,
            games_played=0,
        )
    )

    opponent = make_pool_opponent_fn(pool, mode="uniform")
    test_env = DigimonEnv()
    test_env.reset(seed=0)
    action = opponent(test_env)
    assert isinstance(action, int)
    assert 0 <= action < ACTION_SPACE_SIZE
