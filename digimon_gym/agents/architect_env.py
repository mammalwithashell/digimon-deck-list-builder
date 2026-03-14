"""Gymnasium environment for deck optimization via card swaps.

Models deck building as a finite-horizon MDP where the agent iteratively
swaps cards in/out of a deck to maximise weighted win rate against a
meta-game field.  Uses exponential reward amplification (Q-DeckRec style)
so that improvements at high win rates are disproportionately valuable.

No DB imports. No auth. Pure engine + simulation.
"""

from __future__ import annotations

import math
import logging
from typing import Dict, List, Tuple

import gymnasium
import numpy as np
from gymnasium import spaces

from digimon_gym.agents.architect_pool import CandidatePool
from digimon_gym.agents.architect_simulator import DeckSimulator
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.enums import CardKind

logger = logging.getLogger(__name__)


class DeckBuildingEnv(gymnasium.Env):
    """Deck optimization as an MDP.

    State: deck composition vector + meta weights vector + step counter
    Action: Discrete -- swap one card (or no-op to end episode early)
    Reward: Exponential win-rate delta
    """

    metadata = {"render_modes": []}

    def __init__(
        self,
        pool: CandidatePool,
        simulator: DeckSimulator,
        opponents: List[Tuple[List[str], float]],
        base_deck: List[str],
        max_swaps: int = 10,
        reward_amplification: float = 10.0,
    ):
        """Initialise the deck-building environment.

        Args:
            pool: CandidatePool defining the action space.
            simulator: DeckSimulator for win-rate computation.
            opponents: List of (opponent_deck_ids, meta_weight) for evaluation.
            base_deck: Starting deck (flat card ID list including eggs).
            max_swaps: Maximum number of swap steps per episode.
            reward_amplification: The 'b' constant in exp(b * win_rate).
        """
        super().__init__()

        self._pool = pool
        self._simulator = simulator
        self._opponents = opponents
        self._base_deck = list(base_deck)
        self._max_swaps = max_swaps
        self._reward_amp = reward_amplification

        # Separate egg deck (fixed) from main deck
        db = CardDatabase()
        self._egg_deck: List[str] = []
        main_ids: List[str] = []
        for cid in self._base_deck:
            entity = db.get_card(cid)
            if entity is not None and entity.card_kind == CardKind.DigiEgg:
                self._egg_deck.append(cid)
            else:
                main_ids.append(cid)
        self._base_main_counts: Dict[str, int] = pool.deck_list_to_counts(main_ids)

        # Normalised meta weights vector
        raw_weights = np.array([w for _, w in self._opponents], dtype=np.float32)
        weight_sum = raw_weights.sum()
        if weight_sum > 0:
            self._meta_weights = raw_weights / weight_sum
        else:
            self._meta_weights = np.ones(len(self._opponents), dtype=np.float32) / max(len(self._opponents), 1)

        n_candidates = pool.n_candidates
        n_opponents = len(self._opponents)
        obs_size = n_candidates + n_opponents + 1

        self.observation_space = spaces.Box(
            low=0.0, high=1.0, shape=(obs_size,), dtype=np.float32
        )
        self.action_space = spaces.Discrete(pool.action_size)

        # Episode state (initialised properly in reset)
        self._current_counts: Dict[str, int] = {}
        self._step_counter: int = 0
        self._win_rate: float = 0.0
        self._swap_history: List[dict] = []

    # ------------------------------------------------------------------
    # Gymnasium API
    # ------------------------------------------------------------------

    def reset(
        self,
        seed: int | None = None,
        options: dict | None = None,
    ) -> Tuple[np.ndarray, dict]:
        """Reset to base deck and compute initial win rate.

        Returns:
            (observation, info) where info includes 'action_mask'.
        """
        super().reset(seed=seed)

        self._current_counts = dict(self._base_main_counts)
        self._step_counter = 0
        self._swap_history = []

        # Initial win-rate evaluation
        deck_list = self._pool.counts_to_deck_list(self._current_counts, self._egg_deck)
        self._win_rate = self._simulator.evaluate_deck(deck_list, self._opponents)

        obs = self._build_obs()
        info = self._build_info()
        return obs, info

    def step(
        self, action: int
    ) -> Tuple[np.ndarray, float, bool, bool, dict]:
        """Apply a swap action and return the transition.

        Args:
            action: Discrete action index. 0 = no-op (terminate early).

        Returns:
            (observation, reward, terminated, truncated, info)
        """
        # No-op: end episode immediately with zero reward
        if action == 0:
            obs = self._build_obs()
            info = self._build_info()
            return obs, 0.0, True, False, info

        # Decode and apply swap
        swap = self._pool.decode_action(action)
        if swap is None:
            # Should not happen for action != 0, but handle defensively
            obs = self._build_obs()
            info = self._build_info()
            return obs, 0.0, True, False, info

        remove_card, add_card = swap
        old_wr = self._win_rate

        self._current_counts = self._pool.apply_swap(
            self._current_counts, remove_card, add_card
        )

        # Evaluate new deck
        deck_list = self._pool.counts_to_deck_list(self._current_counts, self._egg_deck)
        new_wr = self._simulator.evaluate_deck(deck_list, self._opponents)

        # Exponential reward
        b = self._reward_amp
        reward = math.exp(b * new_wr) - math.exp(b * old_wr)

        # Record swap
        self._swap_history.append({
            "step": self._step_counter,
            "removed": remove_card,
            "added": add_card,
            "win_rate_before": old_wr,
            "win_rate_after": new_wr,
        })

        self._win_rate = new_wr
        self._step_counter += 1

        terminated = self._step_counter >= self._max_swaps

        obs = self._build_obs()
        info = self._build_info()
        return obs, reward, terminated, False, info

    def action_mask(self) -> np.ndarray:
        """Current valid action mask."""
        return self._pool.get_action_mask(self._current_counts)

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def current_deck_counts(self) -> Dict[str, int]:
        """Current main-deck card counts."""
        return dict(self._current_counts)

    @property
    def current_deck_list(self) -> List[str]:
        """Current full deck list (eggs + main deck)."""
        return self._pool.counts_to_deck_list(self._current_counts, self._egg_deck)

    @property
    def last_win_rate(self) -> float:
        """Most recently computed win rate."""
        return self._win_rate

    @property
    def swap_history(self) -> List[dict]:
        """History of swaps performed this episode."""
        return list(self._swap_history)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _build_obs(self) -> np.ndarray:
        """Construct the observation vector.

        Concatenation of:
        1. Deck vector (n_candidates): normalised card counts
        2. Meta weights (n_opponents): normalised opponent weights
        3. Step counter (1): current_step / max_swaps
        """
        deck_vec = self._pool.deck_to_vector(self._current_counts)
        step_frac = np.array(
            [self._step_counter / self._max_swaps], dtype=np.float32
        )
        return np.concatenate([deck_vec, self._meta_weights, step_frac])

    def _build_info(self) -> dict:
        """Build the info dict returned from reset/step."""
        return {
            "action_mask": self.action_mask(),
            "win_rate": self._win_rate,
            "step": self._step_counter,
            "deck_size": sum(self._current_counts.values()),
        }
