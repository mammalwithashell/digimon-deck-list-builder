"""HeadlessGame: agent-vs-agent game runner optimized for RL training."""

from __future__ import annotations
from typing import Any, Dict, List, Optional, Callable
import numpy as np

from digimon_gym.engine.runners.base_runner import BaseGameRunner
from digimon_gym.engine.loggers import SilentLogger, VerboseLogger


class HeadlessGame(BaseGameRunner):
    """Agent-vs-Agent game runner. Uses SilentLogger by default for performance.

    Provides step-by-step control for RL training loops and
    run_until_conclusion for batch simulation.

    Recording is opt-in:
      - record_actions=True: captures the action sequence for replay/debugging.
      - record_tensors=True: also captures 981-float board tensor at each step (ML debugging).
    """

    def __init__(self, deck1_ids: List[str], deck2_ids: List[str],
                 verbose: bool = False,
                 record_actions: bool = False,
                 record_tensors: bool = False):
        recorder = None
        if record_actions or record_tensors:
            from digimon_gym.engine.recording import GameRecorder
            recorder = GameRecorder(record_tensors=record_tensors)

        logger = VerboseLogger() if verbose else SilentLogger()
        super().__init__(deck1_ids, deck2_ids, logger, recorder=recorder)

    def step(self, action_id: int) -> None:
        """Execute a single action (decoded from integer action space).

        This is the primary interface for RL training loops.
        If recording is enabled, captures action context before/after execution.
        """
        if self.game.game_over:
            return

        player_id = self.game.current_player_id

        if self.recorder:
            # Capture tensor BEFORE action (the decision-point state)
            self.recorder.record_tensor(self.game, player_id)
            action_record = self.recorder.record_action(self.game, action_id, player_id)

        self.game.decode_action(action_id, player_id)

        if self.recorder:
            self.recorder.finalize_action(self.game, action_record)

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

    def get_recording(self) -> Optional[Dict[str, Any]]:
        """Return the full recording as a serializable dict, or None if not recording."""
        if self.recorder:
            return self.recorder.to_dict()
        return None

    def _default_action(self) -> int:
        """Fallback policy: always pass."""
        return 62  # ACTION_PASS_TURN
