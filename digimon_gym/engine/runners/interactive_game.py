"""InteractiveGame: mixed human/agent game runner for API-driven play."""

from __future__ import annotations
from typing import List, Optional, Dict, Any, Callable
import numpy as np

from digimon_gym.engine.runners.base_runner import BaseGameRunner
from digimon_gym.engine.loggers import VerboseLogger
from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.recording import GameRecorder


class InteractiveGame(BaseGameRunner):
    """Mixed human/agent game runner for interactive play.

    Always uses VerboseLogger. Supports step-pause semantics:
    run_step() advances the game but pauses when it's a human
    player's turn, returning state for the UI to render.

    Automatically captures initial state (post-shuffle zones, first player)
    so the API can return recording metadata for client-side recording.
    """

    def __init__(self, deck1_ids: List[str], deck2_ids: List[str],
                 player1_type: PlayerType = PlayerType.Agent,
                 player2_type: PlayerType = PlayerType.Agent):
        self._verbose_logger = VerboseLogger()
        # Create a lightweight recorder just for initial state capture
        recorder = GameRecorder(record_tensors=False)
        super().__init__(deck1_ids, deck2_ids, self._verbose_logger, recorder=recorder)
        self.player1_type = player1_type
        self.player2_type = player2_type

    def is_current_player_human(self) -> bool:
        """Check if the current turn player is human."""
        if self.game.turn_player is self.game.player1:
            return self.player1_type == PlayerType.Human
        else:
            return self.player2_type == PlayerType.Human

    def run_step(self, agent_policy_fn: Optional[Callable] = None) -> Dict[str, Any]:
        """Advance the game by one logical step.

        - If current player is Human: returns state immediately (pauses).
        - If current player is Agent: executes agent turn, returns new state.

        Args:
            agent_policy_fn: Callable(game, mask) -> int for agent decisions.
                             If None, agent always passes.

        Returns:
            Game state dictionary (from game.to_json()).
        """
        if self.game.game_over:
            return self.game.to_json()

        if self.is_current_player_human():
            # Pause: return state for UI rendering
            return self.game.to_json()
        else:
            # Agent turn: execute action
            if agent_policy_fn:
                mask = self.get_action_mask()
                action = agent_policy_fn(self.game, mask)
            else:
                action = 62  # pass
            self.step(action)
            return self.game.to_json()

    def step(self, action_id: int) -> None:
        """Execute a single action (from human or agent)."""
        if self.game.game_over:
            return
        self.game.decode_action(action_id, self.game.current_player_id)

    def get_action_mask(self) -> np.ndarray:
        """Return action mask for the current player."""
        mask = self.game.get_action_mask(self.game.current_player_id)
        # Create a copy to prevent accidental modification of the internal buffer
        return np.array(mask, dtype=np.float32)

    def get_state(self) -> Dict[str, Any]:
        """Return current game state as dictionary."""
        return self.game.to_json()

    def get_last_log(self) -> List[str]:
        """Get buffered log messages since last clear."""
        return self._verbose_logger.get_logs()

    def clear_log(self) -> None:
        """Clear the log buffer."""
        self._verbose_logger.clear()

    def get_initial_state_dict(self) -> Dict[str, Any]:
        """Return initial state metadata for client-side recording.

        Contains deck lists, post-shuffle zone orders, first player, and timestamp.
        The client stores this on game creation and accumulates action responses
        to build a complete replay log in localStorage/IndexedDB.
        """
        if self.recorder and self.recorder.initial_state:
            return self.recorder.to_dict()["initial_state"]
        return {}
