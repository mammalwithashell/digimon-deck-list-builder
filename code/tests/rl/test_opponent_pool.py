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


def test_pfsp_sampling_prefers_opponents_that_beat_the_agent(tmp_path):
    """`win_rate_vs_pool` is the opponent's win rate against the agent
    (`record_match`), so PFSP must favour the opponent with the HIGH value."""
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
    assert counts["strong"] > counts["weak"]
    # Floored, so an opponent the agent always beats is still revisited.
    pool.get("weak").win_rate_vs_pool = 0.0
    assert any(
        pool.sample(rng_seed=seed, mode="pfsp", power=1.0).name == "weak"
        for seed in range(2000)
    )


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



def _stub_game_env():
    """Minimal gymnasium env exposing just the game-over state that
    `OpponentWrapper` reads before resetting."""
    import gymnasium

    class _StubEnv(gymnasium.Env):
        observation_space = gymnasium.spaces.Box(0.0, 1.0, shape=(1,))
        action_space = gymnasium.spaces.Discrete(2)

        def __init__(self):
            self.is_game_over = False
            self.winner_id = None
            self.current_player_id = 1

        def reset(self, **kwargs):
            self.is_game_over = False
            self.winner_id = None
            return [0.0], {}

    return _StubEnv()


def test_opponent_wrapper_feeds_finished_games_into_the_pool(tmp_path, monkeypatch):
    """PFSP for `--opponent pool`: each finished game updates the sampled
    opponent's tally; unfinished games and draws are skipped."""
    import digimon_gym.agents.pilot_training as pt

    monkeypatch.setattr(pt, "make_agent_opponent_fn", lambda *a, **k: (lambda env: 0))
    pool = OpponentPool(manifest_path=tmp_path / "pool.json")
    pool.register(OpponentEntry("gen0", "x.zip", "mlp", win_rate_vs_pool=0.5, games_played=0))
    entry = pool.get("gen0")
    opponent = pt.make_pool_opponent_fn(pool, mode="pfsp")
    env = _stub_game_env()
    wrapper = pt.OpponentWrapper(env, opponent_fn=opponent)

    def finish(winner):
        opponent(env)  # the opponent acts, so an entry is sampled for this game
        env.is_game_over, env.winner_id = winner is not False, (winner or None)

    finish(1)  # agent (player 1) won
    wrapper.reset()
    assert (entry.games_played, entry.win_rate_vs_pool) == (1, 0.0)

    finish(False)  # unfinished / truncated -> no result
    wrapper.reset()
    finish(None)  # draw -> no result
    wrapper.reset()
    assert entry.games_played == 1

    finish(2)  # opponent won; the BO3 between-games reset reports too
    wrapper.reset_inner_only()
    assert (entry.games_played, entry.win_rate_vs_pool) == (2, 0.5)
