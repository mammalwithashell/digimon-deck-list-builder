"""Anchored evaluation — measure a candidate policy against FIXED references
(greedy + frozen champions) instead of the training opponent.

Why this exists: the in-run win rate reuses the training opponent, so under
self-play it is degenerate (a mirror, ~50% or worse) and it is not comparable
across training modes. Anchored evaluation plays the candidate against
opponents that do not move with the learner, seat-balanced, so the number
means the same thing for any model. See the `add-model-evaluation-harness`
change.

The candidate is driven as the agent (engine player 1) via ``model.predict``;
the anchor drives player 2 through ``OpponentWrapper`` (greedy policy or a
frozen model's policy). First-player advantage is balanced by alternating the
seed parity each game (``Game::new`` selects the first player by ``seed % 2``),
so a mirror matchup lands near 50% rather than being decided by who moves
first. NOTE: a raw ``DigimonEnv`` without ``OpponentWrapper`` leaves player 2
passive — exactly the trap that makes self-play eval degenerate — so the
opponent MUST go through the wrapper.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Dict, Optional

import numpy as np

from digimon_gym.digimon_gym import DigimonEnv, greedy_policy
from digimon_gym.agents.champion_registry import ChampionRegistry
from digimon_gym.agents.pilot_training import (
    OpponentWrapper, make_agent_opponent_fn, _unwrap_to_digimon_env,
)

# Safety rail so a pathological non-terminating game can't hang a sweep.
MAX_STEPS = 4000


@dataclass
class MatchRecord:
    """Win/loss/draw counts from the candidate (agent / player 1) perspective."""

    wins: int = 0
    losses: int = 0
    draws: int = 0

    @property
    def games(self) -> int:
        return self.wins + self.losses + self.draws

    @property
    def win_rate(self) -> float:
        return self.wins / self.games if self.games else 0.0

    def __add__(self, other: "MatchRecord") -> "MatchRecord":
        return MatchRecord(
            self.wins + other.wins,
            self.losses + other.losses,
            self.draws + other.draws,
        )


def _seat_balanced_seed(base_seed: int, i: int) -> int:
    """A per-game seed whose parity alternates with ``i`` so the first player
    (selected by ``seed % 2``) alternates between the agent and the anchor."""
    return 2 * (base_seed + i) + (i % 2)


def play_one(agent_model, opponent_fn, deck1, deck2, seed: int,
             tensor_profile: str) -> Optional[int]:
    """Play one game: agent (player 1) = ``model.predict``; player 2 driven by
    ``opponent_fn`` via ``OpponentWrapper``. Returns winner_id (1, 2, or None)."""
    env = OpponentWrapper(
        DigimonEnv(deck1=deck1, deck2=deck2, tensor_profile=tensor_profile),
        opponent_fn=opponent_fn,
    )
    obs, _info = env.reset(seed=seed)  # OpponentWrapper.reset resets opponent state
    state = None
    episode_start = np.array([True])
    steps = 0
    done = False
    while not done and steps < MAX_STEPS:
        mask = _unwrap_to_digimon_env(env).action_mask()
        action, state = agent_model.predict(
            obs, state=state, episode_start=episode_start,
            deterministic=True, action_masks=mask,
        )
        obs, _r, terminated, truncated, _info = env.step(int(action))
        episode_start = np.array([False])
        steps += 1
        done = terminated or truncated
    return _unwrap_to_digimon_env(env).winner_id


def play_matchup(agent_model, opponent_fn, deck_pool, n: int, base_seed: int,
                 tensor_profile: str) -> MatchRecord:
    """Play ``n`` seat-balanced games of ``agent_model`` vs ``opponent_fn``.
    Decks are sampled from ``deck_pool`` (both seats) with a seeded RNG; the
    first player alternates each game. Returns an agent-perspective record."""
    rng = np.random.default_rng(base_seed)
    rec = MatchRecord()
    for i in range(n):
        d1 = deck_pool.sample_uniform_archetype(rng).card_ids
        d2 = deck_pool.sample_uniform_archetype(rng).card_ids
        winner = play_one(agent_model, opponent_fn, d1, d2,
                          _seat_balanced_seed(base_seed, i), tensor_profile)
        if winner == 1:
            rec.wins += 1
        elif winner == 2:
            rec.losses += 1
        else:
            rec.draws += 1
    return rec


def evaluate_against_anchors(
    candidate_model,
    candidate_layout_hash: str,
    registry: ChampionRegistry,
    deck_pool,
    n: int = 40,
    base_seed: int = 777,
    tensor_profile: str = "standard_lite_deck_v2",
    include_greedy: bool = True,
) -> Dict[str, MatchRecord]:
    """Score ``candidate_model`` against greedy plus every layout-compatible
    champion in ``registry``, seat-balanced. Returns ``{anchor_name: record}``.
    Champion opponents are loaded via ``make_agent_opponent_fn``."""
    results: Dict[str, MatchRecord] = {}
    if include_greedy:
        results["greedy"] = play_matchup(
            candidate_model, greedy_policy, deck_pool, n, base_seed, tensor_profile)

    for champ in registry.compatible(candidate_layout_hash):
        opponent_fn = make_agent_opponent_fn(champ.weights_path, algorithm=champ.algorithm)
        results[champ.name] = play_matchup(
            candidate_model, opponent_fn, deck_pool, n, base_seed, tensor_profile)
    return results
