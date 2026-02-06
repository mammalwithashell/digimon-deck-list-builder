"""HeadlessGame: agent-vs-agent game runner optimized for RL training.

Mirrors C# Digimon.Core.HeadlessGame.
"""

from __future__ import annotations
from typing import List, Optional, Callable
import numpy as np

try:
    from python_impl.engine.runners.base_runner import BaseGameRunner
    from python_impl.engine.loggers import SilentLogger, VerboseLogger
except ImportError:
    from digimon_gym.engine.runners.base_runner import BaseGameRunner
    from digimon_gym.engine.loggers import SilentLogger, VerboseLogger


class HeadlessGame(BaseGameRunner):
    """Agent-vs-Agent game runner. Uses SilentLogger by default for performance.

    Provides step-by-step control for RL training loops and
    run_until_conclusion for batch simulation.
    """

    def __init__(self, deck1_ids: List[str], deck2_ids: List[str],
                 verbose: bool = False):
        logger = VerboseLogger() if verbose else SilentLogger()
        super().__init__(deck1_ids, deck2_ids, logger)

    def step(self, action_id: int) -> None:
        """Execute a single action (decoded from integer action space).

        This is the primary interface for RL training loops.
        """
        if self.game.game_over:
            return
        self.game.decode_action(action_id, self.game.current_player_id)

    def run_until_conclusion(self, max_turns: int = 200,
                             policy_fn: Optional[Callable] = None) -> int:
        """Run the game to completion using the given policy function.

        Args:
            max_turns: Maximum action steps before forced conclusion.
            policy_fn: Callable(game, mask) -> int that returns an action_id.
                       If None, uses a simple pass-everything policy.

        Returns:
            Winner player_id (1 or 2), or 0 for draw/timeout.
        """
        steps = 0
        while not self.game.game_over and steps < max_turns:
            if policy_fn:
                mask = self.get_action_mask()
                action = policy_fn(self.game, mask)
            else:
                action = self._default_action()
            self.step(action)
            steps += 1

        if not self.game.game_over:
            self.game.declare_winner(self.game.player1)  # Tiebreaker

        return self.game.winner.player_id if self.game.winner else 0

    def get_action_mask(self) -> np.ndarray:
        """Return action mask for the current player as numpy float32 array."""
        mask_list = self.game.get_action_mask(self.game.current_player_id)
        return np.array(mask_list, dtype=np.float32)

    def get_board_tensor(self, player_id: Optional[int] = None) -> np.ndarray:
        """Return board state tensor for the given player (default: current)."""
        pid = player_id if player_id is not None else self.game.current_player_id
        tensor = self.game.get_board_state_tensor(pid)
        return np.array(tensor, dtype=np.float32)

    def get_last_log(self) -> List[str]:
        """Get buffered log messages (only useful in verbose mode)."""
        return self.logger.get_logs()

    def _default_action(self) -> int:
        """Fallback policy: always pass."""
        return 62  # ACTION_PASS_TURN
