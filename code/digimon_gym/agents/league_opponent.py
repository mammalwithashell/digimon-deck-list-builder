"""Coupled (policy, deck) league opponent for ``--opponent league``
(add-deck-specialist-league).

The deck-specialist league needs an opponent that is a *(policy, deck)* pair:
the agent is pinned to its own deck, and each frozen opponent plays **its own**
deck with **its own** policy. The existing ``--opponent pool`` path loads a policy
but leaves the deck to the deck-pool wrapper; ``LeagueOpponentWrapper`` sets the
deck but loads no policy. This module couples both from one round-pool entry.

Two pieces:

- ``LeagueOpponentController`` — the object handed to ``OpponentWrapper`` as its
  ``opponent_fn``. Its current policy is *settable*; the wrapper points it at the
  sampled opponent's policy before the inner env plays the opponent's turns.
- ``LeaguePoolWrapper`` — a ``gymnasium.Wrapper`` placed **outside**
  ``OpponentWrapper`` (and outside ``MatchEnv``, so it samples once per match).
  On ``reset`` it PFSP-samples an entry, loads+caches its policy, sets the
  controller's policy, and injects ``deck1``/``deck2`` via ``options``. On
  terminal ``step`` it updates PFSP win tracking.

Policy loading is injected (``policy_loader``) so this module does not import
``pilot_training`` (avoids a cycle and keeps it unit-testable with a fake loader).
"""
from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Union

import gymnasium
import numpy as np

from digimon_gym.agents.specialist_registry import LeaguePoolEntry

logger = logging.getLogger(__name__)

#: Per-opponent games observed before PFSP leaves uniform weighting (matches
#: ``LeagueOpponentWrapper``).
PFSP_MIN_GAMES = 5

# An opponent policy: callable(env) -> action int, optionally with reset_state().
OpponentFn = Callable[[Any], int]
PolicyLoader = Callable[[str, str], OpponentFn]  # (weights_path, algorithm) -> fn


def load_league_pool(manifest_path: Union[str, Path]) -> List[LeaguePoolEntry]:
    """Load a round-pool manifest (written by ``SpecialistRegistry.write_round_pool``)
    into ``LeaguePoolEntry`` objects."""
    data = json.loads(Path(manifest_path).read_text(encoding="utf-8"))
    return [LeaguePoolEntry(**e) for e in data.get("entries", [])]


class LeagueOpponentController:
    """A settable opponent policy. Handed to ``OpponentWrapper`` as its
    ``opponent_fn``; the league wrapper repoints it each match."""

    def __init__(self) -> None:
        self._policy: Optional[OpponentFn] = None

    def set_policy(self, policy: OpponentFn) -> None:
        self._policy = policy
        # Fresh episode for this opponent — reset any recurrent state.
        reset = getattr(policy, "reset_state", None)
        if reset is not None:
            reset()

    def __call__(self, env: Any) -> int:
        if self._policy is None:
            raise RuntimeError(
                "LeagueOpponentController has no policy set — the LeaguePoolWrapper "
                "must sample (and set) one on reset before the opponent acts"
            )
        return self._policy(env)

    def reset_state(self) -> None:
        if self._policy is not None:
            reset = getattr(self._policy, "reset_state", None)
            if reset is not None:
                reset()


class LeaguePoolWrapper(gymnasium.Wrapper):
    """Samples a coupled (policy, deck) opponent per match for the league.

    Place OUTSIDE ``OpponentWrapper`` (and ``MatchEnv``) in the chain so each
    reset is one match: the same opponent + deck pair span a BO3 match.
    """

    def __init__(
        self,
        env: gymnasium.Env,
        entries: List[LeaguePoolEntry],
        player_deck: List[str],
        controller: LeagueOpponentController,
        policy_loader: PolicyLoader,
        sampling_mode: str = "pfsp",
        power: float = 1.0,
        seed: Optional[int] = None,
    ) -> None:
        super().__init__(env)
        if not entries:
            raise ValueError("LeaguePoolWrapper requires a non-empty opponent pool")
        self.entries = entries
        self.player_deck = player_deck
        self.controller = controller
        self.policy_loader = policy_loader
        self.sampling_mode = sampling_mode
        self.power = power
        self._rng = np.random.default_rng(seed)

        self._cache: Dict[str, OpponentFn] = {}
        self._games: Dict[str, int] = {e.name: 0 for e in entries}
        self._wins: Dict[str, int] = {e.name: 0 for e in entries}  # agent wins
        self._current: Optional[LeaguePoolEntry] = None

    def _weights(self) -> np.ndarray:
        if self.sampling_mode == "pfsp":
            w = []
            for e in self.entries:
                g = self._games.get(e.name, 0)
                if g >= PFSP_MIN_GAMES:
                    wr = self._wins.get(e.name, 0) / g  # agent win rate vs e
                    # Up-weight opponents the agent loses to more.
                    w.append(max(0.01, (1.0 - wr) ** self.power))
                else:
                    w.append(1.0)  # uniform until enough data
        elif self.sampling_mode == "uniform":
            w = [1.0] * len(self.entries)
        else:
            raise ValueError(f"unknown sampling mode: {self.sampling_mode}")
        arr = np.asarray(w, dtype=np.float64)
        arr /= arr.sum()
        return arr

    def _load(self, entry: LeaguePoolEntry) -> OpponentFn:
        if entry.weights_path not in self._cache:
            self._cache[entry.weights_path] = self.policy_loader(
                entry.weights_path, entry.algorithm
            )
        return self._cache[entry.weights_path]

    def reset(self, **kwargs):
        idx = int(self._rng.choice(len(self.entries), p=self._weights()))
        self._current = self.entries[idx]

        # Couple BOTH halves from one entry: policy + deck. Set the policy on the
        # controller BEFORE the inner reset, so OpponentWrapper plays the right
        # opponent this match.
        self.controller.set_policy(self._load(self._current))

        options = kwargs.get("options") or {}
        options["deck1"] = self.player_deck
        options["deck2"] = self._current.deck
        kwargs["options"] = options

        obs, info = self.env.reset(**kwargs)
        info["opponent_name"] = self._current.name
        info["opponent_deck"] = self._current.deck
        return obs, info

    def step(self, action):
        obs, reward, terminated, truncated, info = self.env.step(action)
        if terminated and self._current is not None:
            name = self._current.name
            self._games[name] = self._games.get(name, 0) + 1
            if float(reward) > 0:
                self._wins[name] = self._wins.get(name, 0) + 1
        return obs, reward, terminated, truncated, info

    @property
    def current_opponent(self) -> Optional[LeaguePoolEntry]:
        return self._current
