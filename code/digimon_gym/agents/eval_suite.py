"""Held-out evaluation suite with fixed seeds, decks, and opponents."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Dict, List

import yaml

from digimon_gym.digimon_gym import DigimonEnv, greedy_policy
from digimon_gym.agents.pilot_training import random_policy


EVAL_OPPONENT_POLICIES: Dict[str, Callable[[DigimonEnv], int]] = {
    "greedy": greedy_policy,
    "random": random_policy,
}


@dataclass(frozen=True)
class Matchup:
    name: str
    deck1: List[str]
    deck2: List[str]
    seeds: List[int]


@dataclass(frozen=True)
class EvalCellResult:
    matchup: str
    games_played: int
    wins: int
    losses: int
    draws: int

    @property
    def win_rate(self) -> float:
        return self.wins / self.games_played if self.games_played else 0.0


@dataclass(frozen=True)
class EvalSuiteResult:
    cell_results: Dict[str, EvalCellResult]

    @property
    def overall_win_rate(self) -> float:
        total_wins = sum(c.wins for c in self.cell_results.values())
        total_games = sum(c.games_played for c in self.cell_results.values())
        return total_wins / total_games if total_games else 0.0


@dataclass
class HeldOutEvalSuite:
    version: int
    opponent_policy: str
    games_per_cell: int
    matchups: List[Matchup] = field(default_factory=list)

    @classmethod
    def from_yaml(cls, path: Path) -> "HeldOutEvalSuite":
        data = yaml.safe_load(Path(path).read_text()) or {}
        matchups = [
            Matchup(
                name=m["name"],
                deck1=list(m["deck1"]),
                deck2=list(m["deck2"]),
                seeds=list(m["seeds"]),
            )
            for m in data.get("matchups", [])
        ]
        suite = cls(
            version=int(data["version"]),
            opponent_policy=str(data["opponent_policy"]),
            games_per_cell=int(data["games_per_cell"]),
            matchups=matchups,
        )
        if suite.opponent_policy not in EVAL_OPPONENT_POLICIES:
            raise ValueError(f"unknown eval opponent policy: {suite.opponent_policy}")
        return suite

    def run(
        self,
        agent_fn: Callable[[DigimonEnv], int],
        max_games_per_cell: int | None = None,
    ) -> EvalSuiteResult:
        opponent_fn = EVAL_OPPONENT_POLICIES[self.opponent_policy]
        cells: Dict[str, EvalCellResult] = {}
        limit = max_games_per_cell or self.games_per_cell

        for matchup in self.matchups:
            wins = losses = draws = 0
            seeds = matchup.seeds[:limit]
            for seed in seeds:
                outcome = self._play_one_game(
                    matchup.deck1,
                    matchup.deck2,
                    seed,
                    agent_fn,
                    opponent_fn,
                )
                if outcome == "win":
                    wins += 1
                elif outcome == "loss":
                    losses += 1
                else:
                    draws += 1
            cells[matchup.name] = EvalCellResult(
                matchup=matchup.name,
                games_played=len(seeds),
                wins=wins,
                losses=losses,
                draws=draws,
            )
        return EvalSuiteResult(cell_results=cells)

    @staticmethod
    def _play_one_game(deck1, deck2, seed, agent_fn, opponent_fn) -> str:
        env = DigimonEnv(deck1=deck1, deck2=deck2)
        _obs, _info = env.reset(seed=seed)
        terminated = truncated = False

        while not (terminated or truncated):
            game = env.runner.game
            action = agent_fn(env) if game.current_player_id == 1 else opponent_fn(env)
            _obs, _reward, terminated, truncated, _info = env.step(int(action))

        game = env.runner.game
        if not getattr(game, "game_over", False):
            return "draw"
        winner = getattr(game, "winner", None)
        winner_id = getattr(winner, "player_id", winner)
        if winner_id == 1:
            return "win"
        if winner_id == 2:
            return "loss"
        return "draw"
