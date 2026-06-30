"""Unit tests for the coupled (policy, deck) league opponent (add-deck-specialist-league).

Covers: controller policy coupling, deck+policy injection on reset, PFSP
up-weighting of losing matchups, and policy caching — all with a fake env and a
fake policy loader (no engine / no SB3).
"""
from __future__ import annotations

import json

import gymnasium
import numpy as np
import pytest

from digimon_gym.agents.league_opponent import (
    LeagueOpponentController,
    LeaguePoolWrapper,
    load_league_pool,
)
from digimon_gym.agents.specialist_registry import LeaguePoolEntry


class _FakeEnv(gymnasium.Env):
    def __init__(self):
        self.observation_space = gymnasium.spaces.Box(low=0.0, high=1.0, shape=(1,), dtype=np.float32)
        self.action_space = gymnasium.spaces.Discrete(2)
        self.last_options = None
        self._reward = 0.0
        self._terminated = False

    def reset(self, *, seed=None, options=None):
        self.last_options = options
        return np.zeros(1, dtype=np.float32), {}

    def step(self, action):
        return np.zeros(1, dtype=np.float32), self._reward, self._terminated, False, {}


class _FakePolicy:
    """callable(env)->action with a reset_state counter."""

    def __init__(self, action: int):
        self.action = action
        self.resets = 0

    def __call__(self, env):
        return self.action

    def reset_state(self):
        self.resets += 1


def _entries():
    return [
        LeaguePoolEntry(name="st-1@r0", weights_path="a.zip", algorithm="mlp", deck="ST-1 Gaia Red"),
        LeaguePoolEntry(name="st-2@r0", weights_path="b.zip", algorithm="mlp", deck="ST-2 Cocytus Blue"),
    ]


def test_controller_requires_policy_then_couples():
    ctrl = LeagueOpponentController()
    with pytest.raises(RuntimeError, match="no policy set"):
        ctrl(_FakeEnv())
    pol = _FakePolicy(action=1)
    ctrl.set_policy(pol)
    assert ctrl(_FakeEnv()) == 1
    assert pol.resets == 1  # set_policy resets recurrent state for the new episode


def test_reset_injects_deck_and_sets_policy():
    entries = _entries()
    loaded = {e.weights_path: _FakePolicy(action=i) for i, e in enumerate(entries)}
    calls = []

    def loader(path, algo):
        calls.append(path)
        return loaded[path]

    ctrl = LeagueOpponentController()
    env = _FakeEnv()
    wrap = LeaguePoolWrapper(
        env, entries, player_deck=["X"], controller=ctrl,
        policy_loader=loader, sampling_mode="uniform", seed=0,
    )
    _, info = wrap.reset()
    cur = wrap.current_opponent
    # deck1 pinned to the agent, deck2 = the SAMPLED opponent's deck
    assert env.last_options["deck1"] == ["X"]
    assert env.last_options["deck2"] == cur.deck
    assert info["opponent_deck"] == cur.deck
    # the controller now drives the sampled opponent's policy
    assert ctrl(env) == loaded[cur.weights_path].action


def test_policy_loader_cached_per_weights_path():
    entries = _entries()
    calls = []

    def loader(path, algo):
        calls.append(path)
        return _FakePolicy(action=0)

    ctrl = LeagueOpponentController()
    wrap = LeaguePoolWrapper(
        _FakeEnv(), entries, player_deck=["X"], controller=ctrl,
        policy_loader=loader, sampling_mode="uniform", seed=1,
    )
    for _ in range(20):
        wrap.reset()
    # at most one load per distinct weights_path despite many resets
    assert len(set(calls)) <= 2
    assert len(calls) == len(set(calls))


def test_pfsp_upweights_losing_matchup():
    entries = _entries()
    ctrl = LeagueOpponentController()
    wrap = LeaguePoolWrapper(
        _FakeEnv(), entries, player_deck=["X"], controller=ctrl,
        policy_loader=lambda p, a: _FakePolicy(0), sampling_mode="pfsp", seed=0,
    )
    # Simulate history: agent BEATS st-1 (10/10) but LOSES to st-2 (0/10).
    wrap._games = {"st-1@r0": 10, "st-2@r0": 10}
    wrap._wins = {"st-1@r0": 10, "st-2@r0": 0}
    w = wrap._weights()
    # st-2 (the losing matchup) must carry far more sampling weight.
    assert w[1] > w[0]
    assert w[1] > 0.9


def test_step_tracks_pfsp_terminal_wins():
    entries = _entries()
    ctrl = LeagueOpponentController()
    env = _FakeEnv()
    wrap = LeaguePoolWrapper(
        env, entries, player_deck=["X"], controller=ctrl,
        policy_loader=lambda p, a: _FakePolicy(0), sampling_mode="uniform", seed=2,
    )
    wrap.reset()
    name = wrap.current_opponent.name
    env._terminated, env._reward = True, 1.0  # agent win
    wrap.step(0)
    assert wrap._games[name] == 1
    assert wrap._wins[name] == 1


def test_load_league_pool_roundtrip(tmp_path):
    manifest = {
        "version": 1,
        "for_deck": "ST-1 Gaia Red",
        "entries": [
            {"name": "st-1@r0", "weights_path": "a.zip", "algorithm": "mlp",
             "deck": "ST-1 Gaia Red", "win_rate_vs_pool": 0.5, "games_played": 0},
        ],
    }
    p = tmp_path / "pool.json"
    p.write_text(json.dumps(manifest))
    entries = load_league_pool(p)
    assert len(entries) == 1 and entries[0].deck == "ST-1 Gaia Red"


def test_load_cache_bounded_lru_evicts():
    """Per-worker opponent cache is bounded (LRU) so concurrent specialists can't
    OOM by each accumulating the whole pool (the EOFError/SIGKILL root cause)."""
    entries = _entries()  # a.zip (e0), b.zip (e1)
    calls = []

    def loader(path, algo):
        calls.append(path)
        return _FakePolicy(action=0)

    wrap = LeaguePoolWrapper(
        _FakeEnv(), entries, player_deck=["X"], controller=LeagueOpponentController(),
        policy_loader=loader, sampling_mode="uniform", seed=0, cache_size=1,
    )
    wrap._load(entries[0])          # load a  -> calls=[a]
    wrap._load(entries[1])          # load b, evict a -> calls=[a,b]
    assert len(wrap._cache) == 1    # bounded
    wrap._load(entries[0])          # a was evicted -> reload -> calls=[a,b,a]
    assert calls == ["a.zip", "b.zip", "a.zip"]
    assert len(wrap._cache) == 1    # still bounded


def test_load_cache_no_reload_within_capacity():
    """Within cache_size, repeated loads are served from cache (no reload)."""
    entries = _entries()
    calls = []

    def loader(path, algo):
        calls.append(path)
        return _FakePolicy(action=0)

    wrap = LeaguePoolWrapper(
        _FakeEnv(), entries, player_deck=["X"], controller=LeagueOpponentController(),
        policy_loader=loader, sampling_mode="uniform", seed=0, cache_size=2,
    )
    for _ in range(5):
        wrap._load(entries[0]); wrap._load(entries[1])   # both fit in cache
    assert calls == ["a.zip", "b.zip"]   # each loaded exactly once
