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
import json
import os
from pathlib import Path
import time
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
             tensor_profile: str, collector=None) -> Optional[int]:
    """Play one game: agent (player 1) = ``model.predict``; player 2 driven by
    ``opponent_fn`` via ``OpponentWrapper``. Returns winner_id (1, 2, or None)."""
    failure_dir = os.environ.get("DIGIMON_EVAL_FAILURE_DIR")
    inner = DigimonEnv(deck1=deck1, deck2=deck2, tensor_profile=tensor_profile,
                       record_actions=bool(failure_dir))
    env = OpponentWrapper(
        inner,
        opponent_fn=opponent_fn,
    )
    if collector is not None:
        inner.card_eval_collector = collector
    first_game = len(collector.games) if collector is not None else 0
    try:
        obs, _info = env.reset(seed=seed)  # reset opponent recurrent state
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
        if not inner.is_game_over:
            raise RuntimeError(f"Anchored evaluation hit a safety cap without a terminal outcome (seed {seed})")
        outcome = inner.terminal_outcome()
        if outcome.get("reason") in {"step_limit", "crash"}:
            raise RuntimeError(f"Anchored evaluation ended with {outcome['reason']} (seed {seed})")
        return _unwrap_to_digimon_env(env).winner_id
    except Exception as exc:
        if collector is not None:
            # Env may already have finalized an engine safety termination.
            collector.finish_game(None, "evaluation_end")
            if len(collector.games) > first_game:
                collector.games[-1]["evaluation_error"] = str(exc)
                collector.games[-1]["terminal_reason"] = "evaluation_end"
        if failure_dir:
            folder = Path(failure_dir)
            folder.mkdir(parents=True, exist_ok=True)
            stem = f"seed-{seed}-{time.time_ns()}"
            (folder / f"{stem}.context.json").write_text(json.dumps({
                "seed": seed, "profile": tensor_profile, "deck1": deck1, "deck2": deck2,
                "error": str(exc)}, indent=2), encoding="utf-8")
            (folder / f"{stem}.recording.json").write_text(
                json.dumps(inner.get_recording(), indent=2), encoding="utf-8")
        raise
    finally:
        if collector is not None:
            inner.card_eval_collector = None
            collector.finish_game(None, "evaluation_end")
        close = getattr(env, "close", None)
        if close is not None:
            close()


def play_matchup(agent_model, opponent_fn, deck_pool, n: int, base_seed: int,
                 tensor_profile: str, collector=None, anchor_name="") -> MatchRecord:
    """Play ``n`` seat-balanced games of ``agent_model`` vs ``opponent_fn``.
    Decks are sampled from ``deck_pool`` (both seats) with a seeded RNG; the
    first player alternates each game. Returns an agent-perspective record."""
    rng = np.random.default_rng(base_seed)
    rec = MatchRecord()
    for i in range(n):
        first = deck_pool.sample_uniform_archetype(rng)
        second = deck_pool.sample_uniform_archetype(rng)
        d1, d2 = first.card_ids, second.card_ids
        extra = {}
        if collector is not None:
            collector.context = dict(anchor=anchor_name,
                panel=collector.provenance.get("panel", ""),
                evaluation_mode="policy_only",
                agent_archetype=first.archetype_name, opponent_archetype=second.archetype_name,
                agent_deck_id=first.deck_id, opponent_deck_id=second.deck_id)
            extra["collector"] = collector
        winner = play_one(agent_model, opponent_fn, d1, d2,
                          _seat_balanced_seed(base_seed, i), tensor_profile, **extra)
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
    collector=None,
) -> Dict[str, MatchRecord]:
    """Score ``candidate_model`` against greedy plus every layout-compatible
    champion in ``registry``, seat-balanced. Returns ``{anchor_name: record}``.
    Champion opponents are loaded via ``make_agent_opponent_fn``."""
    results: Dict[str, MatchRecord] = {}
    def collect_for(name):
        return {"collector": collector, "anchor_name": name} if collector is not None else {}
    if include_greedy:
        results["greedy"] = play_matchup(
            candidate_model, greedy_policy, deck_pool, n, base_seed, tensor_profile, **collect_for("greedy"))

    for champ in registry.compatible(candidate_layout_hash):
        opponent_fn = make_agent_opponent_fn(champ.weights_path, algorithm=champ.algorithm)
        results[champ.name] = play_matchup(
            candidate_model, opponent_fn, deck_pool, n, base_seed, tensor_profile, **collect_for(champ.name))
    return results
