"""League opponent wrapper for meta-weighted and PFSP opponent sampling."""

from __future__ import annotations

import logging
from typing import Any, Dict, List, Optional

import gymnasium
import numpy as np

logger = logging.getLogger(__name__)


class LeagueOpponentWrapper(gymnasium.Wrapper):
    """Samples opponents from a pool during training.

    On each reset(), picks an opponent from the pool using probability weights,
    and injects their deck into the env via options["deck2"].

    Two sampling modes:
    - meta_weighted: Weights = DigiLab meta_share
    - pfsp: Inverse-win-rate weighting (targets weakest matchups)
    """

    def __init__(
        self,
        env: gymnasium.Env,
        agent_pool: List[Dict[str, Any]],
        player_deck: List[str],
        sampling_mode: str = "meta_weighted",
        seed: Optional[int] = None,
    ) -> None:
        """
        Args:
            env: The base DigimonEnv (wrapped by OpponentWrapper)
            agent_pool: List of dicts with keys: agent_id, weights_path, algorithm, deck, weight
            player_deck: Card IDs for the training agent
            sampling_mode: "meta_weighted" or "pfsp"
            seed: Random seed
        """
        super().__init__(env)
        self.agent_pool = agent_pool
        self.player_deck = player_deck
        self.sampling_mode = sampling_mode
        self._rng = np.random.default_rng(seed)

        # Track per-opponent win rates for PFSP
        self._opponent_games: Dict[str, int] = {e["agent_id"]: 0 for e in agent_pool}
        self._opponent_wins: Dict[str, int] = {e["agent_id"]: 0 for e in agent_pool}

        self._current_opponent: Optional[Dict[str, Any]] = None
        self._current_opponent_fn = None

    def _compute_weights(self) -> np.ndarray:
        """Compute sampling weights based on mode."""
        if self.sampling_mode == "pfsp":
            # Inverse win-rate weighting: target opponents we perform worst against
            weights = []
            for entry in self.agent_pool:
                aid = entry["agent_id"]
                games = self._opponent_games.get(aid, 0)
                wins = self._opponent_wins.get(aid, 0)
                if games >= 5:
                    wr = wins / games
                    # Higher weight for opponents we lose to more
                    weights.append(max(0.01, 1.0 - wr))
                else:
                    weights.append(1.0)  # Uniform until enough data
        else:
            # meta_weighted: use the provided weights
            weights = [max(0.01, entry.get("weight", 0.01)) for entry in self.agent_pool]

        arr = np.array(weights, dtype=np.float64)
        arr /= arr.sum()
        return arr

    def reset(self, **kwargs):
        # Sample opponent
        weights = self._compute_weights()
        idx = self._rng.choice(len(self.agent_pool), p=weights)
        self._current_opponent = self.agent_pool[idx]

        # Set decks
        options = kwargs.get("options") or {}
        options["deck1"] = self.player_deck
        options["deck2"] = self._current_opponent["deck"]
        kwargs["options"] = options

        # The actual opponent policy loading is handled by the OpponentWrapper
        # which should be configured with the correct opponent function.
        # This wrapper handles deck sampling; the training worker handles
        # loading the right opponent model.

        obs, info = self.env.reset(**kwargs)
        info["opponent_agent_id"] = self._current_opponent["agent_id"]
        info["opponent_archetype"] = self._current_opponent.get("archetype", "unknown")
        return obs, info

    def step(self, action):
        obs, reward, terminated, truncated, info = self.env.step(action)

        # Track PFSP stats on episode end
        if terminated and self._current_opponent is not None:
            aid = self._current_opponent["agent_id"]
            self._opponent_games[aid] = self._opponent_games.get(aid, 0) + 1
            if float(reward) > 0:
                self._opponent_wins[aid] = self._opponent_wins.get(aid, 0) + 1

        return obs, reward, terminated, truncated, info

    @property
    def current_opponent(self) -> Optional[Dict[str, Any]]:
        return self._current_opponent
