"""Elo ladder — rank a set of policies (a run's checkpoints + frozen champions
+ the greedy anchor) on one comparable scale via a seat-balanced round-robin.

The greedy anchor is profile-agnostic and held at a fixed rating, so it pins
the scale and bridges observation-profile cohorts (model-vs-model is only valid
within a shared tensor layout). The full matchup matrix is retained so that
non-transitive (rock-paper-scissors) dynamics and forgetting — a later
checkpoint losing to an earlier one — are visible rather than hidden behind the
scalar ratings.

Rating math is a Bradley-Terry MLE fit on the Elo scale (a model i beats j with
probability ``1 / (1 + 10**((Rj-Ri)/400))``). Part of
add-model-evaluation-harness.
"""
from __future__ import annotations

import math
from dataclasses import dataclass, field
from typing import Callable, Dict, List, Optional

ELO_SCALE = 400.0
DEFAULT_RATING = 1000.0
_C = ELO_SCALE / math.log(10.0)  # Elo points per natural-log-odds


# --------------------------------------------------------------------------
# Data types
# --------------------------------------------------------------------------

@dataclass
class PlayerSpec:
    """A ladder participant: a model checkpoint/champion or the greedy anchor."""

    name: str
    kind: str  # "model" | "greedy"
    weights_path: Optional[str] = None
    algorithm: str = "lstm"
    layout_hash: Optional[str] = None  # None for greedy (profile-agnostic)
    order: Optional[int] = None        # e.g. checkpoint step, for upset detection


@dataclass
class MatchupCell:
    """Seat-balanced result of a pair, from player A's perspective."""

    a_name: str
    b_name: str
    a_wins: int
    b_wins: int
    draws: int = 0

    @property
    def games(self) -> int:
        return self.a_wins + self.b_wins + self.draws

    @property
    def a_score(self) -> float:
        """A's score share in [0, 1] (win=1, draw=0.5)."""
        return (self.a_wins + 0.5 * self.draws) / self.games if self.games else 0.0


@dataclass
class Upset:
    later: str
    earlier: str
    later_score: float


# --------------------------------------------------------------------------
# Cohort gating
# --------------------------------------------------------------------------

def cohort_compatible(a: PlayerSpec, b: PlayerSpec) -> bool:
    """Two players can play iff they share an observation-profile cohort.
    Greedy (profile-agnostic) bridges all cohorts; two models must share a
    tensor-layout hash."""
    if a.kind == "greedy" or b.kind == "greedy":
        return True
    return a.layout_hash is not None and a.layout_hash == b.layout_hash


# --------------------------------------------------------------------------
# Rating fit (Bradley-Terry MLE on the Elo scale)
# --------------------------------------------------------------------------

def _expected(r_a: float, r_b: float) -> float:
    return 1.0 / (1.0 + 10.0 ** ((r_b - r_a) / ELO_SCALE))


def compute_elo(
    cells: List[MatchupCell],
    anchor: Optional[str] = None,
    anchor_rating: float = DEFAULT_RATING,
    base_rating: float = DEFAULT_RATING,
    k: float = 24.0,
    max_iters: int = 5000,
    tol: float = 1e-7,
) -> Dict[str, float]:
    """Fit Elo ratings from pairwise results by gradient ascent on the
    Bradley-Terry log-likelihood. If ``anchor`` is given its rating is held
    fixed (pinning the scale); otherwise ratings are mean-centred on
    ``base_rating``."""
    names = sorted({c.a_name for c in cells} | {c.b_name for c in cells})
    ratings: Dict[str, float] = {n: base_rating for n in names}
    if anchor in ratings:
        ratings[anchor] = anchor_rating

    for _ in range(max_iters):
        grad: Dict[str, float] = {n: 0.0 for n in names}
        weight: Dict[str, float] = {n: 0.0 for n in names}
        for c in cells:
            if c.games == 0:
                continue
            ea = _expected(ratings[c.a_name], ratings[c.b_name])
            # BT MLE gradient: observed score - expected, scaled by games.
            resid = (c.a_score - ea) * c.games
            grad[c.a_name] += resid
            grad[c.b_name] -= resid
            weight[c.a_name] += c.games
            weight[c.b_name] += c.games

        max_delta = 0.0
        for n in names:
            if n == anchor or weight[n] == 0.0:
                continue
            delta = k * grad[n] / weight[n]
            ratings[n] += delta
            max_delta = max(max_delta, abs(delta))
        if max_delta < tol:
            break

    if anchor not in ratings and names:
        # No fixed anchor: centre the mean on base_rating for identifiability.
        shift = base_rating - sum(ratings.values()) / len(names)
        ratings = {n: r + shift for n, r in ratings.items()}
    return ratings


def elo_standard_errors(
    cells: List[MatchupCell], ratings: Dict[str, float]
) -> Dict[str, float]:
    """Fisher-information-based standard error per rating (Elo points). More
    games and more balanced matchups tighten the estimate; a player with no
    games has infinite SE."""
    info: Dict[str, float] = {n: 0.0 for n in ratings}
    for c in cells:
        if c.games == 0:
            continue
        ea = _expected(ratings.get(c.a_name, DEFAULT_RATING),
                       ratings.get(c.b_name, DEFAULT_RATING))
        contrib = c.games * ea * (1.0 - ea)
        if c.a_name in info:
            info[c.a_name] += contrib
        if c.b_name in info:
            info[c.b_name] += contrib
    return {n: (_C / math.sqrt(v) if v > 0 else math.inf) for n, v in info.items()}


def find_upsets(cells: List[MatchupCell], order: Dict[str, int]) -> List[Upset]:
    """Flag pairs where the later (higher-``order``) player scores < 50%
    against the earlier one — the signature of forgetting / cycling."""
    upsets: List[Upset] = []
    for c in cells:
        if c.a_name not in order or c.b_name not in order or c.games == 0:
            continue
        if order[c.a_name] == order[c.b_name]:
            continue
        if order[c.a_name] > order[c.b_name]:
            later, earlier, score = c.a_name, c.b_name, c.a_score
        else:
            later, earlier, score = c.b_name, c.a_name, 1.0 - c.a_score
        if score < 0.5:
            upsets.append(Upset(later=later, earlier=earlier, later_score=score))
    return upsets


# --------------------------------------------------------------------------
# Round-robin orchestration (uses the engine via anchored_eval.play_matchup)
# --------------------------------------------------------------------------

def run_round_robin(
    players: List[PlayerSpec],
    deck_pool,
    n: int,
    base_seed: int,
    tensor_profile: str,
    load_model: Callable[[PlayerSpec], object],
    opponent_fn_for: Callable[[PlayerSpec], object],
    log: Optional[Callable[[str], None]] = None,
) -> List[MatchupCell]:
    """Play every cohort-compatible pair seat-balanced. For a (greedy, model)
    pair the model is the predict-driven agent (greedy can't predict); for two
    models, A is the agent and B the opponent. Results are A-perspective."""
    from digimon_gym.agents.anchored_eval import play_matchup

    cells: List[MatchupCell] = []
    for i in range(len(players)):
        for j in range(i + 1, len(players)):
            pa, pb = players[i], players[j]
            if pa.kind == "greedy" and pb.kind == "greedy":
                continue
            if not cohort_compatible(pa, pb):
                continue
            # The agent must be a model; keep greedy as the opponent.
            agent, opp = (pb, pa) if pa.kind == "greedy" else (pa, pb)
            if log:
                log(f"  {agent.name} vs {opp.name} (n={n})")
            rec = play_matchup(
                load_model(agent), opponent_fn_for(opp),
                deck_pool, n, base_seed, tensor_profile)
            cells.append(MatchupCell(
                a_name=agent.name, b_name=opp.name,
                a_wins=rec.wins, b_wins=rec.losses, draws=rec.draws))
    return cells
