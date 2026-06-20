"""Per-round league evaluation: the deck matchup matrix + anchored promotion
(add-deck-specialist-league, Group 5).

The in-run win rate is degenerate (rule 30); the league's standing metric is a
**seat-balanced** deck-by-deck matchup matrix plus per-specialist anchored win
rates vs greedy. This reuses the anchored-eval primitives (`play_one`,
`_seat_balanced_seed`, `MatchRecord`) — the difference from `play_matchup` is
that decks are **fixed per seat** (each specialist pilots its own deck) instead
of sampled from a pool.

Model/policy loaders are injected so this is unit-testable without real `.zip`s:
- ``model_loader(weights_path) -> sb3_model`` (agent seat; uses ``.predict``)
- ``policy_loader(weights_path, algorithm) -> fn`` (opponent seat; via OpponentWrapper)
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Dict, List, Optional, Union

from digimon_gym.agents.anchored_eval import (
    MatchRecord,
    _seat_balanced_seed,
    greedy_policy,
    play_one,
)
from digimon_gym.agents.pilot_training import make_agent_opponent_fn
from digimon_gym.agents.specialist_registry import SpecialistRegistry

#: A candidate round-r specialist is promoted over round-(r-1) only if it wins at
#: least this share of a seat-balanced head-to-head (the registry's gating rule).
PROMOTION_THRESHOLD = 0.55


def play_fixed_matchup(
    agent_model,
    opponent_fn,
    deck1: List[str],
    deck2: List[str],
    n: int,
    base_seed: int,
    tensor_profile: str,
) -> MatchRecord:
    """``n`` seat-balanced games with **fixed** decks (agent pilots ``deck1``,
    opponent pilots ``deck2``); first player alternates each game."""
    rec = MatchRecord()
    for i in range(n):
        winner = play_one(
            agent_model, opponent_fn, deck1, deck2,
            _seat_balanced_seed(base_seed, i), tensor_profile,
        )
        if winner == 1:
            rec.wins += 1
        elif winner == 2:
            rec.losses += 1
        else:
            rec.draws += 1
    return rec


def _default_policy_loader(weights_path: str, algorithm: str):
    return make_agent_opponent_fn(weights_path, algorithm=algorithm)


def build_matchup_matrix(
    registry: SpecialistRegistry,
    deck_cards: Dict[str, List[str]],
    tensor_layout_hash: str,
    model_loader: Callable[[str], object],
    n: int = 24,
    base_seed: int = 777,
    tensor_profile: str = "standard_lite_deck_v2",
    policy_loader: Callable[[str, str], Callable] = _default_policy_loader,
    include_greedy: bool = True,
) -> Dict[str, Dict[str, dict]]:
    """Seat-balanced deck-by-deck matrix over the registry's *current* specialists.

    ``matrix[agent_deck][opp_deck] = {"win_rate", "games"}`` is the agent-deck
    specialist (its policy + its deck) vs the opp-deck specialist (its policy +
    its deck). A ``"greedy"`` column gives the per-specialist skill floor. Only
    layout-compatible specialists with known decks are included.
    """
    decks = [
        d for d in registry.decks()
        if registry.get(d).tensor_layout_hash == tensor_layout_hash and d in deck_cards
    ]
    models = {d: model_loader(registry.get(d).weights_path) for d in decks}
    policies = {
        d: policy_loader(registry.get(d).weights_path, registry.get(d).algorithm)
        for d in decks
    }

    matrix: Dict[str, Dict[str, dict]] = {}
    for ad in decks:
        row: Dict[str, dict] = {}
        if include_greedy:
            rec = play_fixed_matchup(
                models[ad], greedy_policy, deck_cards[ad], deck_cards[ad],
                n, base_seed, tensor_profile,
            )
            row["greedy"] = {"win_rate": rec.win_rate, "games": rec.games}
        for od in decks:
            rec = play_fixed_matchup(
                models[ad], policies[od], deck_cards[ad], deck_cards[od],
                n, base_seed, tensor_profile,
            )
            row[od] = {"win_rate": rec.win_rate, "games": rec.games}
        matrix[ad] = row
    return matrix


def write_matchup_matrix(
    path: Union[str, Path], matrix: Dict[str, Dict[str, dict]], round: int
) -> Path:
    out = Path(path)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(
        json.dumps({"version": 1, "round": round, "matrix": matrix}, indent=2),
        encoding="utf-8",
    )
    return out


def should_promote(candidate_record: MatchRecord,
                   threshold: float = PROMOTION_THRESHOLD) -> bool:
    """Promote a round-r candidate over the previous round iff it wins at least
    ``threshold`` of a seat-balanced head-to-head against it. No games => no."""
    if candidate_record.games <= 0:
        return False
    return candidate_record.win_rate >= threshold


def anchored_head_to_head(
    candidate_model,
    previous_policy_fn,
    deck_cards: List[str],
    n: int = 24,
    base_seed: int = 999,
    tensor_profile: str = "standard_lite_deck_v2",
) -> MatchRecord:
    """Seat-balanced mirror head-to-head of a deck's round-r candidate vs its
    round-(r-1) policy (both piloting the same deck). Feeds ``should_promote``."""
    return play_fixed_matchup(
        candidate_model, previous_policy_fn, deck_cards, deck_cards,
        n, base_seed, tensor_profile,
    )
