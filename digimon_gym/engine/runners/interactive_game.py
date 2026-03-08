"""InteractiveGame: mixed human/agent game runner for API-driven play."""

from __future__ import annotations
from typing import List, Optional, Dict, Any, Callable
import time
import numpy as np

from digimon_gym.engine.runners.base_runner import BaseGameRunner
from digimon_gym.engine.loggers import EventLogger
from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.recording import GameRecorder

ACTION_PASS_TURN = 62


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
                 player2_type: PlayerType = PlayerType.Agent,
                 player1_policy: str = "greedy",
                 player2_policy: str = "greedy",
                 agent_action_delay_ms: int = 350,
                 player1_model_path: Optional[str] = None,
                 player2_model_path: Optional[str] = None):
        self._verbose_logger = EventLogger()
        # Create a lightweight recorder just for initial state capture
        recorder = GameRecorder(record_tensors=False)
        super().__init__(deck1_ids, deck2_ids, self._verbose_logger, recorder=recorder)
        self.player1_type = player1_type
        self.player2_type = player2_type
        self.player1_policy = player1_policy.lower()
        self.player2_policy = player2_policy.lower()
        self.agent_action_delay_ms = max(0, int(agent_action_delay_ms))

        # ONNX policies for "trained" agents (lazy-loaded)
        self._onnx_policies: Dict[int, Any] = {}
        if self.player1_policy == "trained" and player1_model_path:
            self._load_onnx_policy(1, player1_model_path)
        if self.player2_policy == "trained" and player2_model_path:
            self._load_onnx_policy(2, player2_model_path)

        # Reset LSTM state to ensure clean episode boundary (CLAUDE.md rule 10)
        self.reset_policies()

    def _load_onnx_policy(self, player_num: int, model_path: str) -> None:
        """Load an ONNX policy for a player (auto-detects MLP vs LSTM)."""
        from digimon_gym.engine.onnx_policy import load_onnx_policy
        self._onnx_policies[player_num] = load_onnx_policy(model_path)

    def reset_policies(self) -> None:
        """Reset ONNX LSTM hidden state. Call at episode/game boundaries."""
        for policy in self._onnx_policies.values():
            if hasattr(policy, "reset"):
                policy.reset()

    @staticmethod
    def _random_policy(mask: np.ndarray) -> int:
        valid = np.where(mask > 0.5)[0]
        if len(valid) == 0:
            return ACTION_PASS_TURN
        return int(np.random.choice(valid))

    @staticmethod
    def _sanitize_action(mask: np.ndarray, action: int) -> int:
        valid = np.where(mask > 0.5)[0]
        if len(valid) == 0:
            return ACTION_PASS_TURN
        valid_set = set(int(v) for v in valid)
        if action in valid_set:
            return action

        # Prefer a non-pass fallback so the agent keeps progressing.
        for candidate in valid:
            if int(candidate) != ACTION_PASS_TURN:
                return int(candidate)
        return int(valid[0])

    def _select_agent_action(self, mask: np.ndarray) -> int:
        # Use current_player_id to pick the right policy (respects active_player during selections)
        is_p1 = self.game.current_player_id == self.game.player1.player_id
        policy_name = self.player1_policy if is_p1 else self.player2_policy
        player_num = 1 if is_p1 else 2

        if policy_name == "trained" and player_num in self._onnx_policies:
            obs = np.array(
                self.game.get_board_state_tensor(self.game.current_player_id),
                dtype=np.float32,
            )
            action = self._onnx_policies[player_num].predict(obs, mask)
        elif policy_name == "random":
            action = self._random_policy(mask)
        else:
            from digimon_gym.digimon_gym import greedy_policy

            class _PolicyEnv:
                def __init__(self, game_ref, mask_ref):
                    self.game = game_ref
                    self._mask = mask_ref

                def get_action_mask(self):
                    return self._mask

            action = int(greedy_policy(_PolicyEnv(self.game, mask)))

        return self._sanitize_action(mask, int(action))

    def is_current_player_human(self) -> bool:
        """Check if the player who must act now is human.

        Uses game.current_player_id which respects active_player
        (set during selection phases when a non-turn player must respond).
        """
        if self.game.current_player_id == self.game.player1.player_id:
            return self.player1_type == PlayerType.Human
        else:
            return self.player2_type == PlayerType.Human

    def run_step(self, agent_policy_fn: Optional[Callable] = None) -> Dict[str, Any]:
        """Advance the game by one logical step.

        - If current player is Human: returns state immediately (pauses).
        - If current player is Agent: executes agent actions until a human turn
          begins or the game ends.

        Args:
            agent_policy_fn: Callable(game, mask) -> int for agent decisions.
                             If None, uses per-player configured policies.

        Returns:
            Game state dictionary (from game.to_ui_json()).
        """
        if self.game.game_over:
            return self.game.to_ui_json()

        if self.is_current_player_human():
            # Pause: return state for UI rendering
            return self.game.to_ui_json()

        # Agent loop: continue until human turn or game over.
        max_agent_actions = 512
        if self.player1_type == PlayerType.Agent and self.player2_type == PlayerType.Agent:
            max_agent_actions = 1
        actions_taken = 0

        while (not self.game.game_over
               and not self.is_current_player_human()
               and actions_taken < max_agent_actions):
            if self.agent_action_delay_ms > 0:
                time.sleep(self.agent_action_delay_ms / 1000.0)
            mask = self.get_action_mask()

            if agent_policy_fn:
                chosen = int(agent_policy_fn(self.game, mask))
                action = self._sanitize_action(mask, chosen)
            else:
                action = self._select_agent_action(mask)

            self.step(action)
            actions_taken += 1

        return self.game.to_ui_json()

    def surrender(self, player_id: int) -> Dict[str, Any]:
        """Handle a player surrendering."""
        self.game.surrender(player_id)
        return self.game.to_ui_json()

    def step(self, action_id: int) -> None:
        """Execute a single action (from human or agent)."""
        if self.game.game_over:
            return
        self.game.decode_action(action_id, self.game.current_player_id)

    def get_action_mask(self) -> np.ndarray:
        """Return action mask for the current player."""
        mask_list = self.game.get_action_mask(self.game.current_player_id)
        return np.array(mask_list, dtype=np.float32)

    def get_state(self) -> Dict[str, Any]:
        """Return current game state as dictionary."""
        return self.game.to_json()

    def get_last_log(self) -> List[str]:
        """Get buffered log messages since last clear."""
        return self._verbose_logger.get_logs()

    def clear_log(self) -> None:
        """Clear the log buffer."""
        self._verbose_logger.clear()

    def get_last_events(self) -> List[Dict[str, Any]]:
        """Get buffered structured events since last clear."""
        return self._verbose_logger.get_events()

    def clear_events(self) -> None:
        """Clear the event buffer."""
        self._verbose_logger.clear_events()

    def get_initial_state_dict(self) -> Dict[str, Any]:
        """Return initial state metadata for client-side recording.

        Contains deck lists, post-shuffle zone orders, first player, and timestamp.
        The client stores this on game creation and accumulates action responses
        to build a complete replay log in localStorage/IndexedDB.
        """
        if self.recorder and self.recorder.initial_state:
            return self.recorder.to_dict()["initial_state"]
        return {}
